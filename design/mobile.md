<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
-->

# pasejo mobile design

Status: draft, 2026-11-16; phase 1 implemented and phase 2 partly implemented, 2026-11-17. This document proposes Android and iOS apps for pasejo. The server pieces (the app sign-in flow and offline-read reports), the Rust core (`pasejo-match`, `pasejo-mobile` and the signer trait) and the read-only Android app without offline reads are implemented; [Phases](#phases) lists what remains. Platform facts name the documentation or source they were read from on 2026-11-16; those marked unverified need checking before an implementation relies on them.

## Summary

The apps are full devices of a pasejo server. They read and write remote stores, fill credentials into other apps and browsers through each platform's autofill framework, and show TOTP codes. They share the protocol, the verifying client and the credential matcher with the CLI through one Rust crate with generated Kotlin and Swift bindings, so every check the CLI makes on what the server returns runs as the same code on a phone.

Seven decisions shape the design:

1. **Full devices.** An app enrolls as an `age` device like a CLI installation, so it receives full wrappings and can decrypt values. Browser devices are names-only because script in a web page can use their keys; an app's code ships signed and loads no script from the server.
2. **One Rust core.** A new crate, `pasejo-mobile`, wraps `pasejo-protocol`, `pasejo-client` and the matcher behind a small interface for UniFFI. The apps contain the user interface, platform key storage and autofill glue, and no protocol logic.
3. **Android 13 and later, keys protected by secure hardware.** The Android app supports API 33 and later on any vendor's device. Where the device's trusted execution environment implements Curve25519, the device keys are generated and used there; elsewhere they are sealed under a key that lives there. A device whose key store reports software keys is refused.
4. **Browser sign-in with PKCE.** Apps sign in through the system browser with an authorization code and PKCE. The device authorization grant the CLI uses is not meant for smartphones.
5. **Autofill reuses the browser host's two phases.** A fill request matches against the lookup object, which the server audits as one lookup, and the chosen credential is one object fetch, which the server audits as one read. The app verifies which domains a requesting app belongs to itself, through Digital Asset Links.
6. **The platform's trust store.** TLS trusts what the phone trusts, including certificate authorities a device management system installs.
7. **Offline reads only where the organization allows them.** By default every value comes from the server. An organization can allow an offline copy in its membership log; the app then reports reads from the copy when it reconnects, and the audit views name the periods in which such reads could go unreported.

## Goals and non-goals

**Goals:**

- sign-in, enrollment and device approval against a pasejo server;
- listing stores and secret names, and revealing or copying a value;
- TOTP codes;
- filling usernames, passwords and TOTP codes into apps and browsers;
- creating and editing secrets in remote stores;
- approving other devices and running the pending actions a device with full keys can run;
- managed configuration and work profiles on company phones, including phones without Google Play services;
- an offline copy of values in organizations that allow it;
- Android first, iOS from the same core.

**Non-goals:**

- running hooks or syncing file stores;
- age plugins and hardware tokens on the phone;
- Android 12 and earlier;
- a web view or any server-provided script inside the app;
- administration that the CLI or the console already covers, such as SCIM tokens, organization settings and recovery.

## Termux and the CLI

Whether the CLI builds and runs under Termux is unverified. Its Linux-only pieces (Landlock in `cli/sandbox.rs`, the Wayland and X11 clipboard) need checking on Android's kernel and without a display server. A Termux guide is worth writing for people who already work in a terminal, but it does not replace an app:

- an Android autofill service is an app component, so the CLI cannot fill other apps;
- the identity file sits in Termux's app directory, protected only by the device lock;
- clipboard access needs the Termux:API add-on, and the CLI's auto-clear does not use it;
- `age-plugin-yubikey` needs a PC/SC daemon.

The web extension cannot fill on Android either: Firefox for Android does not support `runtime.connectNative` or the `nativeMessaging` permission for installed add-ons (MDN browser-compat-data).

## Components and licenses

| Component | Location | License | Role |
|---|---|---|---|
| `pasejo-match` | `crates/pasejo-match` | 0BSD | credential matcher, look-alike detection, fill policies and the field grammar, moved out of the CLI |
| `pasejo-mobile` | `crates/pasejo-mobile` | 0BSD | UniFFI interface over `pasejo-client` and `pasejo-match` |
| pasejo for Android | `metio/pasejo-android` | 0BSD | Kotlin app: user interface, key sealing, autofill and credential provider services |
| pasejo for iOS | `metio/pasejo-ios` | 0BSD | Swift app: user interface, key sealing, credential provider extension |

`pasejo-match` and `pasejo-mobile` live in this workspace and take the workspace version, so a protocol change and its binding change land in one commit. The apps live in their own repositories because their toolchains (Gradle with the Android SDK and NDK, Xcode) and release channels differ from the workspace's, as the web extension's do. Each app release pins one `pasejo-mobile` version. The apps are 0BSD like the CLI, which places no conditions on distribution through app stores.

## Rust core

### Interface

UniFFI generates Kotlin and Swift bindings from Rust (0.32.2 on crates.io). Exported plain functions become blocking functions in Kotlin, which the apps call from a background dispatcher; Rust-side state that the platform supplies, such as sealed keys, crosses the boundary as foreign traits implemented in Kotlin or Swift.

`pasejo-mobile` keeps no state on disk. The app passes in what `pasejo-client` callers already hold and stores what comes back, encrypted, in its private storage:

| Call | Inputs | Result |
|---|---|---|
| `server_info` | URL | version, checkpoint key, OIDC availability |
| `start_app_login`, `finish_app_login` | device keys | authorization URL with PKCE; then a session (see [Sign-in](#sign-in)) |
| `enroll`, `enrollment_status` | session, organization | pending or active |
| `org_log` | session, cached log, pinned genesis | verified log, extended incrementally |
| `stores` | verified log | store identifiers and labels |
| `names` | store, cached index, pin, key ring | names and the new pin |
| `reveal` | store, name, key ring | secret text, zeroized when the binding releases it |
| `totp` | store, name, key ring, time | code and remaining seconds |
| `query` | store, web domain or verified app domains | matches without secret material |
| `fill` | store, web domain or verified app domains, name | credential, released only for a match |
| `commit` | store, base, changes | new index version, retried on conflict like the CLI's funnel |
| `actions`, `run_action` | organization | pending actions this device can run |
| `offline_read_periods` | verified log | periods in which the organization allowed offline reads |
| `report_offline_reads` | store, journal of reads | nothing; the server records one event per read |

`pasejo_protocol::envelope::Signer` puts the signing operation behind a trait that `pasejo_client::DeviceIdentity` holds, so a foreign implementation signs in KeyMint and an in-memory key serves the CLI and the sealed tier (see [Device keys](#device-keys)). `pasejo-mobile` exposes the calls above as methods of one `Device` object, which holds the client, the verified log, the opened key rings and the pins, and returns its state as JSON for the app to store; the state carries no private key.

### Transport

`pasejo-client` uses `ureq` with its default features, which verify TLS through rustls with ring and bundled roots. `Client::with_roots` replaces the bundled roots with DER certificates the caller passes, and the apps pass the platform's trust store, so a self-hosted server behind a company certificate authority works when the phone trusts that authority.

On Android, the app reads every certificate of `AndroidCAStore`, which holds system and user certificates, and passes them as the trust anchors when it creates or restores the device. A device management system installs certificate authorities as user certificates (`DevicePolicyManager.installCaCert`), so they are trusted. rustls then verifies the chain itself, so Android's revocation checks and the app's Network Security Config do not apply; the config therefore cannot pin or restrict trust, and an organization that wants trust limited to its own authority needs a custom rustls verifier instead (an open question below). `rustls-platform-verifier` would add Android's revocation checks at the cost of a Kotlin component built from its Maven archive and a JNI initialization; the app does without it for now. On iOS the Security framework's trust store includes configuration profiles' authorities (unverified).

Trusting user certificates also trusts an authority the person installed themselves. A connection intercepted that way exposes session tokens and metadata, but a token authorizes nothing without the device's request proof, and every log, index and object the server returns is verified against pinned keys, so interception can deny service and observe access patterns without forging data.

### Matcher

The browser host's matcher (`src/browser/matcher.rs`, `lookalike.rs`, `policy.rs`) and the field grammar (`src/exporters/parser.rs`) move into `pasejo-match`, which the CLI and `pasejo-mobile` both use. The matcher takes a `PasswordStore` today; in the crate it takes secret texts and an optional TOTP entry per name, so it does not depend on the CLI's store model. The test that pins `find_matches` and `reveal` to the same secrets moves with it.

Apps add one match input, the requesting app, which [App associations](#app-associations) turns into verified domains before the matcher runs.

## Device keys

### Hardware tiers

The Android app requires API 33. Android 13 added Curve25519 to KeyMint version 2 for the trusted execution environment, signing with Ed25519 and agreeing keys with X25519, but not every Android 13 device has it:

- The compatibility definition for Android 13 to 16 still allows Keymaster 3 and 4 and KeyMint version 1, and none of those versions mention Curve25519 (CDD 9.11 C-1-6).
- The vendor test suite requires KeyMint version 2 only where the vendor API level is 33 or higher (`VSR-3.10-008` in `KeyMintTest.cpp`), which normally means the device launched with Android 13 or later. A device upgraded to Android 13 on an older vendor image has no such requirement.
- On KeyMint version 1 and Keymaster devices, Android's key store generates Curve25519 keys in a software KeyMint without telling the app (source-read, `keystore2` `km_compat.rs`).
- StrongBox implementations support P-256 only, so Curve25519 keys never live in StrongBox (`IKeyMintDevice.aidl`).

The app therefore runs in one of two tiers, chosen at enrollment:

| | Hardware tier | Sealed tier |
|---|---|---|
| When | `hasSystemFeature(FEATURE_HARDWARE_KEYSTORE, 200)` | every other Android 13 device |
| Signing key | Ed25519 generated in KeyMint (`KeyPairGenerator` `"EC"` with `ECGenParameterSpec("ed25519")`, verified on API 36), purpose `SIGN` only | Ed25519 generated in Rust, sealed with AES-256-GCM under a KeyMint key |
| age identity | X25519 generated in KeyMint (`"XDH"`, `ECGenParameterSpec("x25519")`), purpose `AGREE_KEY` only | X25519 generated in Rust, sealed with RSA-3072 OAEP to a KeyMint key whose private half needs authentication per use; sealing needs only the public half, so enrollment asks for no authentication |
| Where the private key is used | inside the trusted execution environment; X25519 returns only the shared secret to the app | in app memory while unsealed |
| Authentication for the age identity | `setUserAuthenticationParameters(15, AUTH_BIOMETRIC_STRONG or AUTH_DEVICE_CREDENTIAL)`; API 36.1 would allow timeout 0 and `BiometricPrompt.CryptoObject(KeyAgreement)`, which the app does not use yet | timeout 0 and `BiometricPrompt.CryptoObject(Cipher)`, available on every supported version |

After generating each key, the app reads `KeyInfo.getSecurityLevel()` (API 31) and refuses to enroll unless every key, the sealing keys included, reports `SECURITY_LEVEL_TRUSTED_ENVIRONMENT` or `SECURITY_LEVEL_STRONGBOX`. Sealing keys prefer StrongBox where it exists. Keys are excluded from backup; a new phone is a new device. Sources: `KeyGenParameterSpec.Builder`, `KeyInfo`, `PackageManager.FEATURE_HARDWARE_KEYSTORE` and `BiometricPrompt.CryptoObject` on developer.android.com; `IKeyMintDevice.aidl`, `KeyMintTest.cpp` and `keystore2` on android.googlesource.com.

The two tiers trade differently. The hardware tier keeps both private keys out of app memory, but before API 36.1 an X25519 key can only be authorized for a time window, and one authentication opens every time-bound key of the user for that window. The sealed tier binds every unseal of the age identity to one authentication, but the unsealed identity sits in app memory while it unwraps keys. Which tier enrolled is visible to the person in the app; whether approvers should see it too is an open question.

### Using the keys

Both public keys come out of the platform as SubjectPublicKeyInfo; the raw 32 bytes are the last 32 bytes of `getEncoded()`. `XDHPublicKey.getU()` is not usable because it reads the little-endian bytes as a big-endian integer (source-read, `AndroidKeyStoreXDHPublicKey.java`).

In the hardware tier the age identity is a Rust `age::Identity` that unwraps X25519 stanzas with the agreement done by Kotlin through a foreign trait: the trait returns the shared secret for the stanza's ephemeral share, and Rust derives the wrapping key with HKDF-SHA-256 and opens the file key as age's X25519 recipient type defines. The app unwraps the epoch keys it needs right after one authentication.

The signing key carries `setUnlockedDeviceRequired(true)` only from API 35. On Android 12 to 14 that flag has documented bugs: such keys could not be used without a secure lock screen, were deleted when the lock screen was removed, and were not reauthorized by some biometric unlocks.

The app keeps the unwrapped key ring in memory for an unlock period, five minutes by default, capped by managed configuration and never beyond the session, and drops it when the period ends, the screen locks or the system removes the app from memory. Values are fetched per reveal, so the key ring holds no secret values.

### Signed message size

KeyMint signs at most 16 KiB with an Ed25519 key and fails with `INVALID_INPUT_LENGTH` beyond that (`IKeyMintDevice.aidl`). Request proofs and token claims sign hashes and short strings. Membership log entries grow with the device lists of `store.epoch` entries, and index envelopes sign the encrypted index itself, which outgrows 16 KiB at roughly a hundred entries (estimated from the entry format: a 32-character identifier, a version and a 43-character hash per name, encrypted and base64-encoded; not measured).

Every envelope therefore signs its context and the SHA-256 of its payload instead of the payload (see "Signed envelopes" in [design/server.md](server.md)), so signed messages stay under 64 bytes and the hardware tier can sign any envelope.

### iOS

The Secure Enclave holds no Curve25519 keys; CryptoKit's `SecureEnclave` offers P-256 and, from iOS 26, ML-KEM and ML-DSA. The iOS app therefore uses the sealed tier: keys in Keychain items with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` for the signing key and `SecAccessControl` with `.biometryCurrentSet` or `.userPresence`, without a reuse duration, for the age identity. Sources: `SecAccessControlCreateWithFlags` and "Accessing keychain items with Face ID or Touch ID" on developer.apple.com.

## Sign-in

RFC 8628 states that the device authorization grant "is not intended to replace browser-based OAuth in native apps on capable devices like smartphones", and RFC 8252 requires native apps to use an external user agent with PKCE. Apps therefore sign in through their own server flow, modelled on the console's and implemented in `crates/pasejo-server/src/auth/flows.rs`:

1. The app calls `pasejo_client::Client::start_app_login` and opens the returned URL, `<server>/auth/app/start?challenge=<S256 challenge>&redirect_uri=wtf.metio.pasejo%3A%2Fauth&state=<state>`, in the system browser: an Auth Tab or Custom Tab on Android, `ASWebAuthenticationSession` on iOS.
2. The server runs its OIDC relying-party flow with the organization's identity provider, as it does for the console.
3. On the callback, the server redirects to the redirect URI with a single-use code that lives ten minutes and the app's state. The URI must be one of the server's `[apps] redirect_uris`, by default only the private-use scheme `wtf.metio.pasejo:/auth`. A private-use scheme is used because one app build serves every self-hosted server, and verified HTTPS app links require each server's domain to be named in the app.
4. `Client::finish_app_login` checks the state and posts the code, the PKCE verifier and a `token/v1` claim with purpose `app`, signed by the device key, to `POST /v1/auth/app`. The server returns a session bound to that key.

The server stores only the code's hash and the challenge, spends the code on the first exchange, and refuses a code whose verifier does not match, which defends against another app registering the same scheme and intercepting the redirect. The server's sign-in suite covers a wrong verifier, a reused code, an expired code, a session used with another key, malformed parameters and an unlisted redirect URI.

## Enrollment

1. The app fetches `GET /v1/server` and pins the checkpoint key on first use, as the CLI does, unless managed configuration supplies it.
2. For each organization, the person compares the organization's fingerprint with one an admin shows them, and the app pins the genesis hash. Managed configuration can supply the genesis hash instead.
3. The app enrolls and shows its device fingerprint, together with the `pasejo org approve` command an approver runs.
4. Another device of the same person, or an approver, signs `device.add`. The app can approve its own later siblings once it is active.
5. A device with full keys runs the `wrap-keys` action, after which the app opens its wrappings.

## Managed devices

Company phones are managed by a device policy controller (DPC), either as the owner of a work profile or as the owner of the whole device. The app works under any DPC on stock Android and on GrapheneOS, and depends on no Google Play services: autofill, the Credential Manager provider on Android 14 and later, managed configurations and the key store are platform APIs.

### Managed configuration

The app declares its restrictions in `res/xml/app_restrictions.xml` (`android.content.APP_RESTRICTIONS`), reads them with `RestrictionsManager.getApplicationRestrictions()` on start and resume, and listens for `ACTION_APPLICATION_RESTRICTIONS_CHANGED`. Source: "Set up managed configurations" on developer.android.com.

| Key | Type | Effect |
|---|---|---|
| `server_url` | `string` | the server; the setup screen shows it read-only |
| `organization` | `string` | the organization to join after sign-in |
| `genesis_hash` | `string` | the organization's pinned genesis, replacing the person's fingerprint comparison |
| `checkpoint_key` | `string` | the server's pinned checkpoint key |
| `max_unlock_seconds` | `integer` | the longest unlock period the person may choose |
| `allow_copy` | `bool` | whether values can be copied to the clipboard |

A pin from managed configuration carries the same trust as a fingerprint an admin reads out: an administrator of the device management system who pushes another organization's genesis enrolls the phone there. Device approval is unchanged; the approver still compares the device's fingerprint, which the app shows.

A DPC delivers these values with `DevicePolicyManager.setApplicationRestrictions`, a platform call that needs nothing from Google Play on the phone. Device management consoles that read an app's restriction schema do so through Google Play's APIs or Play's managed-configurations frame, so on stock Android the app needs a Google Play listing for those consoles to offer its keys; other consoles take the keys as entered by the administrator.

### Work profiles

An app installed in a work profile is a separate installation with its own keys, so it enrolls as its own device, and the work and personal installations are two devices of the same person. An autofill service fills apps in its own profile only; GrapheneOS declined filling across the profile boundary (GrapheneOS issue 9104). The organization's apps and the pasejo app therefore go into the same profile.

### GrapheneOS

GrapheneOS supports work profiles under any DPC. Device owners work but must be set up through ADB, because its setup wizard has no provisioning; QR provisioning is an open issue whose pull requests were closed unmerged. Google's own device management (Android Device Policy) does not work, because Play services run sandboxed and unprivileged; non-Google products do, and the project names Microsoft's as one example (GrapheneOS issue 7173). No other product was confirmed.

Certificate authorities a DPC installs are user certificates (`DevicePolicyManager.installCaCert`). Apps ignore user certificates by default since Android 7.0, and the platform verifier trusts them (see [Transport](#transport)).

Key attestation can prove that the device keys live in the trusted execution environment and, through the verified boot key in GrapheneOS's signed `attestation.json`, that the phone runs GrapheneOS. Carrying the attestation chain in the enrollment request so an approver's client verifies it is an open question.

## App associations

Before a fill, the app turns the requesting app into domains it has checked itself:

- **Browsers.** The app keeps a list of browsers by package name and signing certificate fingerprint. For a listed browser it trusts `ViewNode.getWebDomain()`; for any other app it ignores the reported web domain, as the `AutofillService` documentation requires.
- **Apps with a statement file.** For a requesting package, the app collects candidate domains: the registrable domains spelled by the package name's labels (`com.example.mail` gives `example.com`) and, not implemented yet, the hosts of `androidapp://` entries in the store's lookup fields. It fetches `https://<domain>/.well-known/assetlinks.json` without redirects, requires `application/json`, follows at most ten includes, and accepts the domain when a statement with relation `delegate_permission/common.get_login_creds` or `delegate_permission/common.handle_all_urls` names the package and the SHA-256 fingerprint of one of its signing certificates. Results are cached for a day. A statement names an exact host; subdomains do not match.
- **Apps without a statement file.** A secret whose URL field names `androidapp://<package>` matches that package once the secret also carries an `android-cert` field with the certificate fingerprint. The lookup object carries `android-cert` with the fill policies, so a query can match on it. Writing the field on the first fill, after showing the package and its certificate, needs writing and comes with phase 3.

The requesting package comes from `AssistStructure.getActivityComponent()`, and its certificates from `PackageManager.getPackageInfo(package, GET_SIGNING_CERTIFICATES)`: the current signers (`SigningInfo.getApkContentsSigners()`), not certificates the app rotated away from. The browser list is Google's allowlist of privileged browsers for Credential Manager (`gpm-passkeys-privileged-apps/apps.json`), embedded in the app. A Credential Manager provider receives the package and its signing information from the framework in `CallingAppInfo`. Whether an autofill service targeting API 30 or later sees the requesting package without a `<queries>` declaration or `QUERY_ALL_PACKAGES` is unverified. Sources: "Statement list syntax" and "Relation strings" on developers.google.com/digital-asset-links; `AutofillService`, `AssistStructure`, `SigningInfo` and `CallingAppInfo` on developer.android.com.

Fetching statement files tells each candidate domain that a pasejo app looked for it; the domain learns no more than a sign-in would tell it.

iOS gives a credential provider service identifiers that the system has already resolved through associated domains, so the iOS app relies on the platform's association.

## Autofill

| | Android | iOS |
|---|---|---|
| Framework | `AutofillService`; `CredentialProviderService` (API 34) for Credential Manager requests, which include passwords and passkeys | credential provider extension, `ASCredentialProviderViewController` (iOS 12; request-based methods from iOS 17) |
| Request names | the requesting package; `ViewNode.getWebDomain()` for browser fields that report it; for Credential Manager, the origin a privileged browser passes | service identifiers: URL, domain or app |
| Locked key ring | `setAuthentication` on the `FillResponse` or `Dataset` with an `IntentSender` to the unlock activity; an `AuthenticationAction` for Credential Manager | the extension cancels with `userInteractionRequired`, and the system calls `prepareInterfaceToProvideCredential` |

Sources: "Build autofill services", `FillResponse.Builder`, `AssistStructure.ViewNode` and "Integrate Credential Manager with your credential provider solution" on developer.android.com; `ASCredentialProviderViewController` on developer.apple.com.

A fill runs the browser host's two phases:

1. **Query.** The app opens the store's lookup object and runs `pasejo-match` for the requested domain or app. The server records one lookup. Results carry names and usernames only.
2. **Fill.** The person picks a result and authenticates if the key ring is closed. The app fetches that one object and releases the credential only if it is still a match for the request. A TOTP code is attached only on an exact-host match, and HOTP entries are never filled, as in the browser host.

The fill and confirm policies stored in secrets apply unchanged: `pasejo-fill: never` hides a secret from autofill, and `pasejo-confirm` makes the app ask before filling. When a query finds nothing, the app names stored look-alike domains as the web extension does.

Passkeys through the Credential Manager provider and the iOS credential provider extension are designed in [design/passkeys.md](passkeys.md).

Browser coverage on Android depends on the browser:

- Chrome fills through a third-party service only after the person selects "Autofill using another service" in Chrome's settings, and Chrome no longer supports the autofill framework's compatibility mode from version 137.
- Firefox's GeckoView supports third-party providers through Android's autofill framework.
- Vanadium on GrapheneOS implements Android's autofill natively, without Google Play services (grapheneos.org/features).

The setup screen explains the Chrome setting. It asks the system to select the app as the autofill service with `ACTION_REQUEST_SET_AUTOFILL_SERVICE`, and on Android 14 and later opens the credential provider settings with `createSettingsPendingIntent()`, because each manufacturer places that screen differently.

iOS lets a provider register credential identities (`ASCredentialIdentityStore`: service identifier, user name and record identifier) so the QuickType bar can offer them before the extension opens. That would hand secret names and usernames to the operating system in a form the system can display, and how the system protects that store is undocumented. The app does not register identities; the person opens the extension from the QuickType bar or the password menu.

## Offline use

The server design fetches every secret value from the server so that every read reaches the audit log; the client cache holds the index, the wrappings and the log head in encrypted form. By default the apps follow that rule: names and labels are available offline from the cache, values are not.

An organization can allow offline reads with `pasejo org settings --offline-reads true`, which records `offline_reads` in the membership log so every client agrees whether it holds. The server side is implemented: `POST /v1/orgs/{org}/stores/{store}/offline-reads` records each reported read as an `object.read.offline` event, refusing reads outside the periods in which the log allowed them, and `pasejo audit log` and the console's audit view name those periods (see "Offline reads" in [design/server.md](server.md)).

While the verified log allows offline reads, the app:

1. Adds each value it fetched from the server to an offline copy, one record per object version, encrypted under a key-store key that needs authentication like the age identity. A value enters the copy only after an audited fetch, so the first read of each version is always recorded.
2. Answers reveals and fills from the copy when the server is unreachable, and appends each such read (object identifier, version, time) to an encrypted local journal.
3. Reports the journal with `report_offline_reads` when it reconnects, and drops journal entries once the server accepts them.

The app deletes the copy when the log turns `offline_reads` off, when its device loses access to a store, and when the person signs out. It learns of the first two only from a log it fetched, so a phone that stays offline keeps its copy and its unreported journal; a lost or compromised phone reports nothing. The warnings in the CLI and the console say so.

## Writing

Creating, editing, moving and removing secrets go through `pasejo_client::store::draft` and a commit, as the CLI's remote funnel does: renames keep identifiers, writes are sealed under the current epoch, and a conflict refetches and retries up to three times before reporting a concurrent change. Generated passwords use the `passwords` crate with the defaults of `secret generate`.

## File stores

A file store is one age-encrypted file synced by hooks. The apps can open such a file read-only through the platform's document picker, for example a file another app keeps in sync, after the person adds the phone's age recipient to the store with `pasejo recipient add`. Writing is out of scope: a phone writing the file would race the desktop's hooks, and conflicts are resolved by the merge driver Git runs. Reading needs `models::password_store` and the store decryption moved out of the CLI into a shared crate, so it comes after the remote-store app.

## Phases

1. **Core.** Signing behind a trait in `pasejo-client`; `pasejo-match` extracted with the CLI's tests passing unchanged; `pasejo-mobile` with UniFFI bindings and tests that drive it against the real server, as `crates/pasejo-server/tests/client.rs` does for the client. The app sign-in flow and offline-read reports are already in the server.
2. **Android, read-only.** Sign-in, enrollment in both key tiers, managed configuration, store and name lists, reveal and copy with clipboard clearing, TOTP, the autofill service with app associations, offline reads. Implemented except offline reads, candidate domains from `androidapp://` entries, and localization beyond English.
3. **Android, writing.** Secret editing, device approval, pending actions and the Credential Manager provider.
4. **iOS.** The same scope on the same core.
5. **File stores.** Read-only import.

## Testing

- `pasejo-mobile` is tested in Rust against the in-process server harness, including the tampering cases the client suite already has (withheld and swapped objects, a rolled-back index, keys without commitments).
- The app sign-in flow and offline-read reports are tested in the server's suites, against the real router and through `pasejo-client`.
- The bindings are exercised from Kotlin and Swift unit tests against a server the test runner starts. The Android app's JVM tests exercise the bindings against the core built for the host, without a server; the server-backed flows are covered in Rust by `crates/pasejo-server/tests/mobile.rs`.
- Android autofill is tested with instrumented tests against a test app with login fields, including a test app whose statement file the test server serves and one signed with another certificate; iOS with UI tests against a test app.
- Managed configuration is tested with a test DPC that sets restrictions through `setApplicationRestrictions`, as owner of a work profile on an emulator.
- The key tiers are tested on emulators for the round trip and on hardware for the security level checks and biometric prompts, which emulators do not reproduce faithfully: at least one device per tier, a Pixel on GrapheneOS, and a device that upgraded to Android 13 on an older vendor image.

## Distribution

- **Android:**
  - F-Droid builds from source and accepts Rust toolchains and NDK builds (Inclusion Policy and Build Metadata Reference on f-droid.org). Reproducible builds are optional; with them F-Droid can publish the APK signed by the project.
  - A signed APK goes on each app release.
  - Google Play needs a developer account. It is optional for people installing the app themselves, and needed on stock Android for device management consoles that read the restriction schema through Google Play.
- **iOS:** the App Store, which needs an Apple Developer Program membership, review of every release, and macOS runners with signing credentials in CI. The workspace's weekly calendar release does not apply to the apps; an app releases when a `pasejo-mobile` change affects it.

## Open questions

- Whether approvers should see a device's key tier, and whether enrollment should carry a key attestation chain that the approver's client verifies.
- A custom rustls verifier restricted to the organization's certificate authority, supplied through managed configuration, for organizations that do not want user certificates trusted.
- How long an offline copy may live without reconnecting before the app deletes it on its own.
- Which device management products to test beyond a test DPC.
