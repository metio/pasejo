<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
-->

# pasejo mobile design

Status: draft, 2026-11-16. This document proposes Android and iOS apps for pasejo. Nothing in it is implemented. Platform facts name the documentation they were read from on 2026-11-16; those marked unverified need checking before an implementation relies on them.

## Summary

The apps are full devices of a pasejo server. They read and write remote stores, fill credentials into other apps and browsers through each platform's autofill framework, and show TOTP codes. They share the protocol, the verifying client and the credential matcher with the CLI through one Rust crate with generated Kotlin and Swift bindings, so every check the CLI makes on what the server returns runs as the same code on a phone.

Five decisions shape the design:

1. **Full devices.** An app enrolls as an `age` device like a CLI installation, so it receives full wrappings and can decrypt values. Browser devices are names-only because script in a web page can use their keys; an app's code ships signed and loads no script from the server.
2. **One Rust core.** A new crate, `pasejo-mobile`, wraps `pasejo-protocol`, `pasejo-client` and the matcher behind a small interface for UniFFI. The apps contain the user interface, platform key storage and autofill glue, and no protocol logic.
3. **Keys sealed by the platform.** The device's signing key and age identity are encrypted under keys of the platform's hardware-backed key store. Opening the age identity requires biometric or device-credential authentication; signing requests requires an unlocked device.
4. **Browser sign-in with PKCE.** Apps sign in through the system browser with an authorization code and PKCE, which needs one new server flow. The device authorization grant the CLI uses is not meant for smartphones.
5. **Autofill reuses the browser host's two phases.** A fill request matches against the lookup object, which the server audits as one lookup, and the chosen credential is one object fetch, which the server audits as one read.

## Goals and non-goals

**Goals:**

- sign-in, enrollment and device approval against a pasejo server;
- listing stores and secret names, and revealing or copying a value;
- TOTP codes;
- filling usernames, passwords and TOTP codes into apps and browsers;
- creating and editing secrets in remote stores;
- approving other devices and running the pending actions a device with full keys can run;
- Android first, iOS from the same core.

**Non-goals:**

- running hooks or syncing file stores;
- age plugins and hardware tokens on the phone;
- an offline copy of secret values (see [Offline use](#offline-use));
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
| `query` | store, web domain or app identity | matches without secret material |
| `fill` | store, web domain or app identity, name | credential, released only for a match |
| `commit` | store, base, changes | new index version, retried on conflict like the CLI's funnel |
| `actions`, `run_action` | organization | pending actions this device can run |

`pasejo_client::DeviceIdentity` holds the signing key in memory. The binding builds it per call from the unsealed key and drops it afterwards, so the signing key exists in Rust memory only while requests are signed.

### Transport

`pasejo-client` uses `ureq` with its default features, which verify TLS through rustls with ring and bundled roots. Self-hosted servers behind a corporate certificate authority need the platform's trust store, which includes user-installed and managed roots. `rustls-platform-verifier` claims Android and iOS support (unverified); the alternative is passing the platform's verification result into Rust through a foreign trait.

### Matcher

The browser host's matcher (`src/browser/matcher.rs`, `lookalike.rs`, `policy.rs`) and the field grammar (`src/exporters/parser.rs`) move into `pasejo-match`, which the CLI and `pasejo-mobile` both use. The matcher takes a `PasswordStore` today; in the crate it takes secret texts and an optional TOTP entry per name, so it does not depend on the CLI's store model. The test that pins `find_matches` and `reveal` to the same secrets moves with it.

Apps add one match input: the requesting app. Android's autofill documentation tells services to verify a requesting browser's signing certificate before trusting the web domain it reports, and to check web domains against the Public Suffix List and Digital Asset Links. An app identity (an Android package name and signing certificate, an iOS app service identifier) matches a store's URL fields through the domains the app is associated with, so stores need no new field. Whether the apps verify those associations by fetching the domain's statement file or rely on the platform's verification is an open question.

## Device keys

An app generates the device's Ed25519 signing key and an age X25519 identity in Rust and hands both to the platform layer, which seals them:

| | Android (API 30 and later) | iOS |
|---|---|---|
| Sealing key | AES-256-GCM key in Android Keystore, StrongBox-backed when available | Keychain items with access control |
| Signing key | sealed under a key with `setUnlockedDeviceRequired(true)` | `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` |
| age identity | sealed under a key with `setUserAuthenticationParameters(0, AUTH_BIOMETRIC_STRONG or AUTH_DEVICE_CREDENTIAL)`, opened through `BiometricPrompt` with a `CryptoObject` | `SecAccessControl` with `.biometryCurrentSet` or `.userPresence`, without a reuse duration |
| Backup | excluded from backup; a new phone is a new device | `ThisDeviceOnly` items do not leave the device |

Sources: `KeyGenParameterSpec.Builder` and `BiometricPrompt.CryptoObject` on developer.android.com; `SecAccessControlCreateWithFlags` and "Accessing keychain items with Face ID or Touch ID" on developer.apple.com.

A timeout of 0 makes the Android key usable for exactly one operation per authentication, and on iOS every read of the item authenticates. The app therefore unseals the age identity once per unlock, unwraps the epoch keys it needs, and drops the identity.

The protocol's keys stay software keys under hardware sealing for two reasons:

- **iOS.** The Secure Enclave holds no Curve25519 keys; CryptoKit's `SecureEnclave` offers P-256 and, from iOS 26, ML-KEM and ML-DSA.
- **Android.** Android 13 (API 33) added Curve25519 to KeyMint for the trusted execution environment, but StrongBox does not support it, the names are undocumented on developer.android.com (`KeyPairGenerator` `"XDH"`, `Signature` `"Ed25519"` in the framework source), and each key serves one purpose. Keeping the signing key in the TEE on API 33 and later is possible; keeping the age identity there would need an age `Identity` implementation that performs the X25519 agreement through the platform. Both are later hardening, not part of the first release.

The app keeps the unwrapped key ring in memory for a configurable unlock period, five minutes by default and never beyond the session, and drops it when the period ends, the screen locks or the system removes the app from memory. Values are fetched per reveal, so the key ring holds no secret values.

## Sign-in

RFC 8628 states that the device authorization grant "is not intended to replace browser-based OAuth in native apps on capable devices like smartphones", and RFC 8252 requires native apps to use an external user agent with PKCE. Apps therefore sign in through a new server flow, modelled on the console's:

1. The app generates a PKCE verifier and opens `<server>/auth/app/start?challenge=<S256 challenge>&device=<device id>` in the system browser: an Auth Tab or Custom Tab on Android, `ASWebAuthenticationSession` on iOS.
2. The server runs its OIDC relying-party flow with the organization's identity provider, as it does for the console.
3. On the callback, the server redirects to the private-use URI scheme `wtf.metio.pasejo:/auth` with a single-use code that lives ten minutes. A private-use scheme is used because one app build serves every self-hosted server, and verified HTTPS app links require each server's domain to be named in the app.
4. The app calls `POST /v1/auth/app` with the code, the PKCE verifier and a `token/v1` envelope signed by its device key, and receives a session bound to that key.

The server stores only the code's hash and the challenge, and refuses a code whose verifier does not match, which defends against another app registering the same scheme and intercepting the redirect. This flow is the one server change the apps need; it lands in `crates/pasejo-server/src/auth/flows.rs` beside the console flow and is tested in the server's sign-in suite.

## Enrollment

1. The app fetches `GET /v1/server` and pins the checkpoint key on first use, as the CLI does.
2. For each organization, the person compares the organization's fingerprint with one an admin shows them, and the app pins the genesis hash.
3. The app enrolls and shows its device fingerprint, together with the `pasejo org approve` command an approver runs.
4. Another device of the same person, or an approver, signs `device.add`. The app can approve its own later siblings once it is active.
5. A device with full keys runs the `wrap-keys` action, after which the app opens its wrappings.

## Autofill

| | Android | iOS |
|---|---|---|
| Framework | `AutofillService` (API 26); `CredentialProviderService` (API 34) for Credential Manager requests, which include passwords | credential provider extension, `ASCredentialProviderViewController` (iOS 12; request-based methods from iOS 17) |
| Request names | the requesting package; `ViewNode.getWebDomain()` for browser fields that report it; for Credential Manager, the origin a privileged browser passes | service identifiers: URL, domain or app |
| Locked key ring | `setAuthentication` on the `FillResponse` or `Dataset` with an `IntentSender` to the unlock activity; an `AuthenticationAction` for Credential Manager | the extension cancels with `userInteractionRequired`, and the system calls `prepareInterfaceToProvideCredential` |

Sources: "Build autofill services", `FillResponse.Builder`, `AssistStructure.ViewNode` and "Integrate Credential Manager with your credential provider solution" on developer.android.com; `ASCredentialProviderViewController` on developer.apple.com.

A fill runs the browser host's two phases:

1. **Query.** The app opens the store's lookup object and runs `pasejo-match` for the requested domain or app. The server records one lookup. Results carry names and usernames only.
2. **Fill.** The person picks a result and authenticates if the key ring is closed. The app fetches that one object and releases the credential only if it is still a match for the request. A TOTP code is attached only on an exact-host match, and HOTP entries are never filled, as in the browser host.

The fill and confirm policies stored in secrets apply unchanged: `pasejo-fill: never` hides a secret from autofill, and `pasejo-confirm` makes the app ask before filling. When a query finds nothing, the app names stored look-alike domains as the web extension does.

Browser coverage on Android depends on the browser:

- Chrome fills through a third-party service only after the person selects "Autofill using another service" in Chrome's settings, and Chrome no longer supports the autofill framework's compatibility mode from version 137.
- Firefox's GeckoView supports third-party providers through Android's autofill framework.

The setup screen explains the Chrome setting.

iOS lets a provider register credential identities (`ASCredentialIdentityStore`: service identifier, user name and record identifier) so the QuickType bar can offer them before the extension opens. That would hand secret names and usernames to the operating system in a form the system can display, and how the system protects that store is undocumented. The app does not register identities; the person opens the extension from the QuickType bar or the password menu.

## Offline use

The server design fetches every secret value from the server so that every read reaches the audit log; the client cache holds the index, the wrappings and the log head in encrypted form. The apps follow the same rule: names and labels are available offline from the cache, values are not.

An offline value cache would trade that audit guarantee for availability on planes and in basements. If organizations ask for it, it belongs in the membership log as an organization setting, so every client agrees whether it is allowed, with cached reads reported to the audit log when the device reconnects. This document does not specify it.

## Writing

Creating, editing, moving and removing secrets go through `pasejo_client::store::draft` and a commit, as the CLI's remote funnel does: renames keep identifiers, writes are sealed under the current epoch, and a conflict refetches and retries up to three times before reporting a concurrent change. Generated passwords use the `passwords` crate with the defaults of `secret generate`.

## File stores

A file store is one age-encrypted file synced by hooks. The apps can open such a file read-only through the platform's document picker, for example a file another app keeps in sync, after the person adds the phone's age recipient to the store with `pasejo recipient add`. Writing is out of scope: a phone writing the file would race the desktop's hooks, and conflicts are resolved by the merge driver Git runs. Reading needs `models::password_store` and the store decryption moved out of the CLI into a shared crate, so it comes after the remote-store app.

## Phases

1. **Core.** `pasejo-match` extracted with the CLI's tests passing unchanged; the app sign-in flow in the server; `pasejo-mobile` with UniFFI bindings and tests that drive it against the real server, as `crates/pasejo-server/tests/client.rs` does for the client.
2. **Android, read-only.** Sign-in, enrollment, store and name lists, reveal and copy with clipboard clearing, TOTP, the autofill service.
3. **Android, writing.** Secret editing, device approval, pending actions and the Credential Manager provider.
4. **iOS.** The same scope on the same core.
5. **File stores.** Read-only import.

## Testing

- `pasejo-mobile` is tested in Rust against the in-process server harness, including the tampering cases the client suite already has (withheld and swapped objects, a rolled-back index, keys without commitments).
- The app sign-in flow is tested in the server's sign-in suite: a wrong verifier, a reused code, an expired code and a claim signed by another key are refused.
- The bindings are exercised from Kotlin and Swift unit tests against a server the test runner starts.
- Android autofill is tested with instrumented tests against a test app with login fields; iOS with UI tests against a test app.
- The sealing layers are tested on emulators for the round trip and on hardware for StrongBox and biometric prompts, which emulators do not reproduce faithfully.

## Distribution

- **Android:**
  - F-Droid builds from source and accepts Rust toolchains and NDK builds (Inclusion Policy and Build Metadata Reference on f-droid.org). Reproducible builds are optional; with them F-Droid can publish the APK signed by the project.
  - A signed APK goes on each app release.
  - Google Play is optional and needs a developer account.
- **iOS:** the App Store, which needs an Apple Developer Program membership, review of every release, and macOS runners with signing credentials in CI. The workspace's weekly calendar release does not apply to the apps; an app releases when a `pasejo-mobile` change affects it.

## Open questions

- Platform trust store or bundled roots for TLS.
- Whether the apps verify app-to-domain associations themselves.
- An offline value cache as an organization setting.
- Android work profiles and managed configuration, so an MDM can push the server URL and organization.
- Keeping the signing key in the Android TEE on API 33 and later.
- Passkeys, which both platforms' credential provider frameworks support and pasejo does not store yet.
