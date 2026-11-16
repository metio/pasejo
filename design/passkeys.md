<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
-->

# pasejo passkeys design

Status: draft, 2026-11-16. This document proposes storing passkeys in pasejo stores and using them from the web extension, the Android app and the iOS app. Nothing in it is implemented. Facts name the specification, documentation or source file they were read from on 2026-11-16; those marked unverified need checking before an implementation relies on them.

## Summary

pasejo can act as a WebAuthn authenticator whose credentials live in its stores, as Bitwarden, 1Password and KeePassXC do. A stored passkey is a new object kind next to secrets and one-time passwords, so its private key never appears in `secret show`, in exports that do not ask for it, or in the lookup object.

Five decisions shape the design:

1. **Synced passkeys.** pasejo's passkeys are multi-device credentials: backup eligible and backed up (BE=1, BS=1), signature counter fixed at 0, attestation format `none` with a pasejo AAGUID, ES256 keys.
2. **A typed object kind.** `passkey` entries sit beside `secrets` and `otp` in file stores and in the encrypted index, named like the login they belong to, as one-time passwords are.
3. **Two-phase use.** A query matches a relying party identifier against the lookup object without key material; an assertion fetches one object and signs, and is audited as one read.
4. **Shared means shared.** A passkey in a store every team member reads is a credential the whole team holds. The design says so wherever a passkey is created in a shared store, and offers a store setting that refuses passkeys.
5. **The signing happens in pasejo.** The web extension builds WebAuthn's client data and checks the relying party identifier against the page's origin; the native host or the app signs. The private key never enters the browser.

## Goals and non-goals

**Goals:**

- creating and using passkeys on websites through the web extension in Firefox and chromium browsers;
- the same from the Android app on Android 14 and later and the iOS app on iOS 17 and later;
- importing passkeys from Bitwarden's JSON export and the FIDO Credential Exchange Format, and exporting them to both;
- passkeys in file stores and remote stores alike.

**Non-goals:**

- acting as a roaming authenticator over CTAP (USB, NFC, hybrid);
- attestation other than `none`;
- algorithms other than ES256 for new passkeys;
- the WebAuthn extensions `prf` and `largeBlob` in the first version.

## What a passkey holds

WebAuthn Level 3 (W3C Recommendation, 25 August 2026, §4 "Public Key Credential Source") defines a credential source as type, credential identifier, private key, relying party identifier, user handle and optional display data. The user handle must be set for discoverable credentials. Implementations persist the same set: Bitwarden's `fido2Credentials` (`credentialId`, `keyValue` as base64url PKCS#8, `rpId`, `userHandle`, `userName`, `userDisplayName`, `counter`, `discoverable`, `creationDate`), KeePassXC's entry attributes, and CXF's `passkey` record (`credentialId`, `rpId`, `username`, `userDisplayName`, `userHandle`, `key`).

A pasejo passkey object holds:

| Field | Form |
|---|---|
| `rp_id` | relying party identifier, a domain |
| `credential_id` | base64url, 16 random bytes for new passkeys |
| `user_handle` | base64url, as the relying party sent it |
| `user_name`, `user_display_name` | text, as the relying party sent it |
| `algorithm` | COSE algorithm; -7 (ES256) for new passkeys, -8 (EdDSA) accepted on import |
| `key` | base64url PKCS#8 private key |
| `created` | milliseconds since the epoch |

The lookup object gains, per passkey name, `rp_id`, `user_name` and `credential_id`, which a query needs and which reveal nothing a relying party does not already show. The key and the user handle stay in the object.

In a file store, `PasswordStore` gains a `passkeys` map beside `secrets` and `otp`. In a remote store, the encrypted index gains a `passkeys` map of entries, and passkey objects are sealed under the epoch's data key like every other object. Both are format additions that older clients must refuse to rewrite rather than silently drop; how an older CLI meets a store with passkeys is an open question.

## Authenticator behaviour

- **Flags.** User presence (UP) is set on every operation, since a person always picks the credential. User verification (UV) is set only when pasejo verified the person for this operation: a biometric or device-credential prompt in the apps. BE and BS are always 1; WebAuthn forbids changing BE after registration (§6.1, §6.1.3), and Bitwarden, KeePassXC and Android's provider sample all emit BE=1 and BS=1.
- **Counter.** Always 0. WebAuthn allows a constant zero, and the relying party skips its clone check when both stored and received counters are zero (§6.1.1, §7.2). Several devices and people use one stored passkey concurrently, so any increasing counter would race and trip that check. CXF requires importers to zero counters and never increment them.
- **Attestation.** Format `none` with an empty statement (§8.7), and a pasejo AAGUID generated once and fixed in `pasejo-protocol`. Relying parties that follow Android's guide store the AAGUID to name the provider.
- **Algorithms.** New passkeys use ES256, which every inspected provider supports and Apple's `ASCOSEAlgorithmIdentifier` is limited to. A registration whose `pubKeyCredParams` does not offer -7 is declined, so the browser or platform falls back to its own authenticator.

## Web extension

The extension becomes the WebAuthn client for pages where pasejo answers. Every inspected manager uses the same mechanism: a script in the page's main world, registered at `document_start` in all frames, replaces `navigator.credentials.create` and `get`, keeps the originals, and calls them when it declines (Bitwarden's `fido2-page-script.ts`, keepassxc-browser's `passkeys.js`). Main-world scripts are available from Chrome 102 and Firefox 128 (mdn/browser-compat-data). Chrome's `webAuthenticationProxy` is not usable: it exists for remote desktop software, takes over every WebAuthn request including security keys once attached, does not report the requesting origin, and is Chromium-only.

A main-world script shares the page's JavaScript environment, so the page can see and interfere with it (MDN, `ExecutionWorld`). The script therefore only relays; every decision runs in the isolated content script, the background and the native host:

1. The content script reads the frame's origin from its own `location`, and the background takes the frame URL from the browser-attested sender, as Bitwarden does.
2. The background checks the request as a WebAuthn client must (WebAuthn L3 §5.1.3, §5.1.4.1): the origin is `https`, or `http` on `localhost`; the RP ID equals the origin's effective domain or is a registrable suffix of it, with the public suffix list; otherwise Related Origin Requests from `https://<RP ID>/.well-known/webauthn` with at least five registrable labels; `create` only when same-origin with its ancestors; the Permissions Policy features `publickey-credentials-create` and `publickey-credentials-get`.
3. The background builds `clientDataJSON` (`type`, `challenge`, `origin`, `crossOrigin`, `topOrigin`) and its SHA-256.
4. The native host receives the origin, the RP ID and the client data hash, repeats the RP ID check against the origin, and signs. It never returns the private key.

The native-messaging protocol gains three actions, under a new `PROTOCOL_VERSION`:

| Action | Request | Reply |
|---|---|---|
| `passkey_query` | `rp_id`, `origin`, optional allowed credential identifiers | matches: store, name, user name, credential identifier; no key material |
| `passkey_get` | `store`, `name`, `rp_id`, `origin`, `client_data_hash`, `user_verified` | authenticator data, signature, user handle, credential identifier |
| `passkey_create` | `store`, `name`, `rp_id`, `origin`, `client_data_hash`, user fields, offered algorithms, excluded credential identifiers | credential identifier, attestation object |

`passkey_get` re-runs the query match as `reveal` does, so the host signs only for a credential the query would have offered for that RP ID, and appends each assertion to the release log with origin and caller before replying. `pasejo-confirm` and `pasejo-fill: never` apply to the login a passkey belongs to.

The desktop host cannot verify the person: it has no biometric prompt, and an identity's plugin PIN or touch is the identity's own policy. It sets UV=0, so relying parties that require user verification refuse the assertion and the extension falls back to the browser's authenticator. Whether to offer verification through pinentry is an open question.

## Android

Android 14 (API 34) introduced the platform Credential Manager and `CredentialProviderService`; on Android 13 Credential Manager needs Google Play services, so the Android app offers passkeys from API 34 only. The provider declares `TYPE_PUBLIC_KEY_CREDENTIAL` in its `android.credentials.provider` metadata and answers in two phases (developer.android.com, "Integrate Credential Manager with your credential provider solution"):

- **Begin.** `onBeginGetCredentialRequest` returns one `PublicKeyCredentialEntry` per match from the lookup object, or an `AuthenticationAction` while the key ring is closed. `onBeginCreateCredentialRequest` returns `CreateEntry` items for the stores the person may write.
- **Selection.** The provider activity receives the request JSON and client data hash with `PendingIntentHandler`, prompts with `BiometricPrompt` and sets UV=1, fetches one object, signs, and answers with the WebAuthn response JSON.

The origin comes from `CallingAppInfo`: for a browser, `getOrigin(allowlist)` returns the web origin only when the caller's package and certificate are on the allowlist, which the app keeps with its browser list; for an app, the origin is `android:apk-key-hash:<base64 SHA-256 of the signing certificate>`, checked against Digital Asset Links as [App associations](mobile.md#app-associations) describes. Google's guide tells providers not to compare the origin with the RP ID themselves, because that refuses valid Related Origin Requests.

Whether Chrome and Brave on GrapheneOS route WebAuthn to a third-party provider without Google Play services is unverified. GrapheneOS states that Vanadium uses Android's Credential Manager natively for passkeys, without sandboxed Google Play (grapheneos.org/features).

## iOS

From iOS 17, a credential provider extension declares `ProvidesPasskeys` and handles `prepareInterface(forPasskeyRegistration:)` and `prepareCredentialList(for:requestParameters:)` (developer.apple.com, AuthenticationServices). The system hands the provider an RP ID and a client data hash; the provider never sees the origin or the client data, so the system's associated-domain checks are the only origin check. Registration completes with an attestation object the provider builds, assertion with authenticator data and a signature.

The provider does not publish `ASPasskeyCredentialIdentity` entries to the identity store, for the reason it publishes no password identities: RP IDs, user names and credential identifiers would be handed to the system outside pasejo's encryption.

## Import and export

- **Bitwarden JSON.** `items[].login.fido2Credentials[]`, all values strings, `keyValue` base64url PKCS#8, `counter` "0". `export bitwarden` writes a login's passkey there; Bitwarden stores one passkey per login item.
- **CXF.** The FIDO Credential Exchange Format's `passkey` credential (Proposed Standard, errata of 9 March 2026). CXF carries no encryption of its own, so pasejo writes a CXF export encrypted with age to the recipients the person names. The Credential Exchange Protocol exists only as a 2024 working draft; Apple's OS-mediated exchange (iOS and macOS 26) is the practical path on Apple platforms.
- **KeePassXC.** Its passkey attributes map field by field; its EdDSA keys import with algorithm -8.

Imports zero any counter and keep BE=1.

## Shared passkeys

1Password and Bitwarden offer passkeys in shared vaults and collections. A relying party sees one credential and learns only that "it was the same user" (WebAuthn L3 §4, "User Verification"); it cannot tell the team members apart. Unlike a password, a passkey cannot be rotated from pasejo: removing a reader rotates the store's epoch, but the reader may have copied the private key, and only registering a new passkey at the relying party and deleting the old one there revokes it.

pasejo therefore:

- warns when a passkey is created in a store that more than one person reads;
- offers a store setting, `passkeys = false` in the encrypted index's store settings, that refuses passkey creation and import in that store;
- lists, in `pasejo store rotation` after a reader is removed, the passkeys the removed reader could read, next to the secrets to rotate.

## Phases

1. **Storage.** The `passkey` object kind in file and remote stores, the lookup fields, the store setting, `pasejo passkey list` and `remove`, import and export, and the rotation listing. Test vectors for the console port.
2. **Web extension.** The page script, the client checks, the native-messaging actions and the release log entries, with e2e tests against a test relying party.
3. **Android.** The Credential Manager provider in the Android app's writing phase.
4. **iOS.** The credential provider extension's passkey methods.

## Testing

- Authenticator data, attestation objects and assertion signatures are checked against an independent WebAuthn relying-party library in the Rust tests.
- The RP ID and origin rules run against WebAuthn's own examples and Related Origin Requests with five and six labels.
- The web extension e2e suite registers and signs in at a test relying party in headless Chromium, and checks that a declined request reaches the browser's own authenticator.
- The host refuses `passkey_get` for a credential whose RP ID does not match the origin, and records every assertion it signs.

## Open questions

- How an older CLI or client treats a store whose index or file carries passkeys.
- Whether the desktop host should verify the person through pinentry and set UV=1.
- Whether passkeys should default to refused in stores read by more than one person.
- Conditional mediation (passkey suggestions in the page's autofill) in the web extension.
- Whether a native macOS app acting as a system passkey provider replaces the extension in Safari.
