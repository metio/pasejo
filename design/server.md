<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
-->

# pasejo server design

Status: accepted, 2026-11-15. This document specifies what `pasejo-server`, its clients and its Kubernetes bridge do, and the implementation follows it. Where the code and this document disagree, the disagreement is a bug in one of them.

## Summary

`pasejo-server` is a self-hosted server through which an organization manages its stores. It adds:

- SSO sign-in and SCIM provisioning;
- organizations with roles and teams;
- remote stores that people, machines and Kubernetes clusters read and write;
- a hash-chained audit log that compliance staff can query and verify;
- an administrator console in the browser;
- a break-glass account for when SSO is unavailable.

The server never holds a key that decrypts a secret, a secret name or a store name. Clients encrypt all of them with keys that are wrapped to individual devices. The server sees:

- users, teams and devices;
- opaque store and secret identifiers and padded sizes;
- which device fetched or wrote which identifier, and when.

Six decisions shape the design:

1. **Remote stores are a second backend.** Remote stores are a second kind of store next to file stores. File stores keep their single-file format, their hooks and their git workflow unchanged.
2. **Encrypted index.** A remote store holds one encrypted object per secret under a random identifier, plus an encrypted index that maps names to identifiers and carries the store's label. The index hides secret names and store names from the server, which the single file does for a file store. It also makes per-secret sync and per-secret audit possible.
3. **Epoch keys.** Each store has epoch keys. The server keeps them wrapped to every authorized device, and every secret object is encrypted under them. A YubiKey user touches the key once per command, and adding a member wraps two keys instead of re-encrypting every secret.
4. **Signed membership log.** A signed membership log decides who may decrypt, and every client verifies it. The log also commits to each epoch's keys, so a server can neither add a recipient nor hand a device keys of its own choosing.
5. **Names-only browser devices.** Browser devices hold non-extractable WebCrypto keys and receive names-only wrappings. Script running in the console can therefore never reveal a secret value, even if the page is compromised.
6. **Hash-chained audit.** Every fetch, every commit and every refusal lands in a hash-chained audit log, with checkpoints signed by the server and optionally co-signed by the organization. Auditors resolve identifiers to names on their own machines.

## Goals and non-goals

Every item under Goals is implemented, and every item under Non-goals is deliberately left out.

**Goals:**

- OIDC sign-in for people and workload-token sign-in for machines;
- organizations, roles, teams and SCIM 2.0 provisioning;
- remote stores with per-secret sync and conflict resolution;
- audit of every write, every server-observed read and every refusal, with export and independent verification;
- revocation with epoch rotation and lists of secrets to rotate;
- time-bound grants and access reviews;
- a bridge that feeds external-secrets-operator;
- an administrator console;
- a break-glass account and an offline recovery recipient;
- deployment as one binary or container image with SQLite or PostgreSQL.

**Non-goals:**

- server-side decryption;
- SAML in the server (SAML-only organizations use an OIDC broker such as Dex or Keycloak);
- replacing git sync for file stores;
- enforcing anything on a client after it has decrypted a value;
- showing or editing secret values in the browser console.

## Components and licenses

| Component | Location | License | Role |
|---|---|---|---|
| `pasejo` | repository root | 0BSD | CLI; gains remote stores and the commands below |
| `pasejo-protocol` | `crates/pasejo-protocol` | 0BSD | wire types, encodings, cryptography, membership-log verification; no I/O |
| `pasejo-client` | `crates/pasejo-client` | 0BSD | blocking HTTP client, device keys, remote store operations |
| `pasejo-server` | `crates/pasejo-server` | AGPL-3.0-or-later | the server and the browser console it serves |
| `pasejo-bridge` | `crates/pasejo-bridge` | 0BSD | Kubernetes bridge for external-secrets-operator |

The shared crates stay 0BSD so that the 0BSD CLI and bridge can link them, and only the server carries the AGPL. The CLI publishes `pasejo-protocol` and `pasejo-client` to crates.io together with itself. The server and the bridge ship as release binaries and as container images under `ghcr.io/metio/pasejo-server` and `ghcr.io/metio/pasejo-bridge`.

## What the server learns

A remote store reveals more metadata than a file store synced through git. The git host of a file store sees one file, its size, commit times and committers. A remote store reveals:

- the number of secrets;
- their padded sizes;
- which identifier each device reads and writes.

That is the price of per-secret audit and per-secret sync. Organizations that do not want to pay it keep file stores.

| Data | Visible to the server |
|---|---|
| Users (OIDC issuer and subject, email, display name), teams and their display names, devices and their public keys | yes: authorization, SSO and SCIM need them |
| Roles and grants | yes |
| Store names | no: the label is inside the encrypted index |
| Number of secrets and OTP entries per store | yes: one object each |
| Size of each secret | padded with Padmé, which leaks O(log log L) bits of a length L |
| Secret names, field names, URLs, values, OTP seeds | no |
| Which identifier a device fetched or wrote, and when | yes |
| Renames | no: a rename changes only the encrypted index |

Access patterns are the main residual leak. Without learning a name, the server can correlate the fetch of one identifier with outside events, such as a deployment that follows it every time.

## Name hiding

The encrypted index with random identifiers was chosen over three alternatives:

- **Identifiers derived from names**, as an HMAC of the name under a store key. A removed member who kept the key can test guessed names against identifiers indefinitely, which confirms that `prod/database` exists. Rejected.
- **The whole store as one remote object.** It hides counts and access patterns, but then every read is a read of everything and the server can audit only "fetched the store". File stores with an upload hook already offer this.
- **Oblivious access (ORAM, PIR).** Its cost is out of proportion to secrets of a few hundred bytes. Rejected.

Store names are hidden the same way, because the label lives inside the index. The server lists a user's stores as identifiers, and clients decrypt each store's index to show its label. Teams take their names from the identity provider, which the server already knows through SCIM, so team names stay visible.

## Encodings

- **Identifiers:** 16 random bytes encoded as 32 lowercase hexadecimal characters. Organizations, users, devices, teams, stores and secrets all use this form.
- **Binary values:** base64url without padding.
- **Times:** milliseconds since the Unix epoch, as unsigned integers.
- **JSON:** UTF-8. Integers stay below 2^53, so JavaScript reads them exactly.

### Signed envelopes

Everything a device signs travels as an envelope:

```json
{"payload": "<base64url of the JSON payload bytes>",
 "signatures": [{"device": "<device id>", "signature": "<base64url>"}]}
```

A signature is Ed25519 over `"pasejo:" ‖ context ‖ 0x00 ‖ payload bytes`. Verifiers check the signature over the exact bytes they received and only then parse them, so the design needs no canonical JSON. The contexts are:

| Context | Signed by | Payload |
|---|---|---|
| `log-entry/v1` | the entry's author, plus co-signers where two admins are required | a membership log entry |
| `index/v1` | the committing device | an index version |
| `request/v1` | the device bound to a session | an API request (see [Request proofs](#request-proofs)) |
| `token/v1` | a device claiming a session | a session exchange |
| `checkpoint/v1` | the server key, and optionally organization devices | an audit checkpoint |

The hash of an envelope is SHA-256 of its payload bytes.

A `token/v1` payload is `{"purpose", "value", "signing_key", "time"}`. `purpose` is `device`, `web`, `workload` or `emergency`; `value` is the device code of a device grant, the base64url SHA-256 of the presented workload JWT, or empty. Binding the JWT's hash into the claim ties a workload session to that token, so a claim cannot be replayed with another one. The server accepts a claim only within 60 seconds of its `time`.

### Padding

Plaintexts are padded with Padmé (Nikitin et al., 2019). The padded plaintext is the 4-byte big-endian length of the real plaintext, the plaintext, and zero bytes up to `padme(4 + length)`, where:

- `E = ⌊log2 L⌋`
- `S = ⌊log2 E⌋ + 1`
- `mask = 2^(E − S) − 1`
- `padme(L) = (L + mask) & ¬mask` for `L ≥ 2`, and `L` otherwise.

## Keys

### Device keys

A device is one installation that holds keys: a CLI, a bridge or a browser. Each device has:

| Key | CLI and bridge devices | Browser devices |
|---|---|---|
| Signing key | Ed25519, generated by pasejo, stored in a mode `0600` file | Ed25519, non-extractable WebCrypto key in IndexedDB |
| Encryption recipient | any age recipient pasejo accepts: X25519, SSH, or an age plugin such as a YubiKey (`kind: age`) | X25519 public key; the private key is a non-extractable WebCrypto key in IndexedDB (`kind: hpke`) |

A device's fingerprint is the first 20 bytes of SHA-256 over `kind ‖ 0x00 ‖ recipient ‖ 0x00 ‖ signing key`, shown as five groups of eight hexadecimal characters. Approvers compare fingerprints in strict mode.

Browser keys require WebCrypto Ed25519 and X25519. According to MDN's compatibility data, Ed25519 shipped in Chrome 137, Firefox 129 and Safari 17, and X25519 in Chrome 133, Firefox 130 and Safari 17. Script in the page can use a non-extractable key but cannot export it. A cross-site scripting flaw could still use the key while the page is open, which is why browser devices receive names-only wrappings.

### Epoch keys

Each store has epochs 0, 1, 2 and so on. Epoch *e* has two random 256-bit keys:

- **the names key `N_e`** decrypts the index, which holds secret names and the store label;
- **the data key `D_e`** decrypts secret objects and the lookup object.

Each epoch's `store.epoch` log entry carries commitments to both keys:

- `names_commit = SHA-256("pasejo names commit v1" ‖ store ‖ epoch ‖ N_e)`
- `data_commit = SHA-256("pasejo data commit v1" ‖ store ‖ epoch ‖ D_e)`

Here store is the 16 identifier bytes and epoch an 8-byte big-endian integer. A client rejects any unwrapped key whose commitment differs from the signed one, so a server cannot slip a device keys it knows.

Epochs are chained backwards. For *e > 0*, the server stores `prev_names` and `prev_data`, which hold `N_{e−1}` and `D_{e−1}` encrypted under keys derived from `N_e` and `D_e`. A holder of epoch *e* reads objects that still carry an older epoch, and a removed member holding *e − 1* learns nothing about *e*.

### Wrappings

A wrapping delivers one epoch's keys to one device. Its plaintext is:

```json
{"store": "<id>", "epoch": 3, "names_key": "<base64url>", "data_key": "<base64url or null>"}
```

- **age devices:** the plaintext is age-encrypted to the device's recipient, so YubiKey and SSH recipients keep working.
- **hpke devices:** the plaintext is sealed with HPKE (RFC 9180) base mode with DHKEM(X25519, HKDF-SHA256), HKDF-SHA256 and AES-256-GCM. The info string is `"pasejo wrapping v1"`. The associated data is the store identifier bytes, the 8-byte epoch and the device identifier bytes. The wrapping is the 32-byte encapsulated key followed by the ciphertext.

The browser opens HPKE with WebCrypto alone (X25519, HMAC-SHA256, AES-GCM), so the console loads no third-party script.

Who gets which wrapping is derived from the membership log for each store and epoch:

| Device | Wrapping |
|---|---|
| age devices of users holding an effective reader, writer or admin grant, directly or through a team | full: names and data key |
| the organization's recovery recipient | full |
| age devices of organization admins and auditors without a full wrapping | names-only |
| hpke devices of every user above | names-only, never full |

The server accepts a wrapping only for a device the log entitles to it, only for the current epoch, and only from a device that itself holds a wrapping of that epoch; a full wrapping needs a full one. Any such device may add a missing wrapping, and only a store admin may replace an existing one, so a reader cannot overwrite another device's keys with garbage.

### Derived keys and ciphers

Every symmetric encryption uses AES-256-GCM with a random 96-bit nonce, written in front of the ciphertext. Keys come from HKDF-SHA256:

| Object | Key derivation (IKM, salt, info) | Associated data |
|---|---|---|
| index | `N_e`, store bytes, `"pasejo index v1"` | `"pasejo index v1" ‖ store ‖ epoch ‖ index version` |
| secret object | `D_e`, store bytes, `"pasejo object v1" ‖ id ‖ version` | `"pasejo object v1" ‖ store ‖ epoch ‖ id ‖ version` |
| lookup object | `D_e`, store bytes, `"pasejo lookup v1"` | `"pasejo lookup v1" ‖ store ‖ epoch ‖ index version` |
| previous names key | `N_e`, store bytes, `"pasejo previous names v1"` | `"pasejo previous v1" ‖ store ‖ epoch` |
| previous data key | `D_e`, store bytes, `"pasejo previous data v1"` | `"pasejo previous v1" ‖ store ‖ epoch` |

Versions and epochs are 8-byte big-endian integers, and identifiers are their 16 bytes. Each secret object version has a key of its own. Binding identifiers, epochs and versions into the associated data stops the server from swapping ciphertexts between objects, stores or versions.

## Remote store objects

### Secret objects

A secret object is identified by a random identifier and a version that grows with each write. Its padded plaintext is one of:

```json
{"kind": "secret", "secret": "<text>"}
{"kind": "otp", "otp": {"secret": "...", "otp_type": "totp", "algorithm": "sha1", "digits": 6, "period": 30, "counter": 0, "skew": 1}}
```

### Index

The index is the signed, chained description of a store's current content. Each version is an envelope with context `index/v1` whose payload is:

```json
{"store": "<id>", "version": 7, "prev": "<base64url hash of version 6, null for version 1>",
 "epoch": 2, "log_head": "<base64url hash of the author's membership log head>",
 "time": 1763251200000, "index": "<base64url ciphertext>", "lookup_hash": "<base64url or null>",
 "changed": ["<id>"], "deleted": ["<id>"]}
```

Its padded plaintext keeps secrets and one-time passwords in separate maps, mirroring the local store's `secrets` and `otp` maps, so the same name can carry both:

```json
{"label": "production",
 "settings": {"uri_keys": ["uri", "url", "link", "site"], "username_keys": ["username", "user", "login", "email"]},
 "secrets": {"db/postgres": {"id": "<id>", "version": 4, "hash": "<base64url SHA-256 of the object ciphertext>"}},
 "otp": {"github.com": {"id": "<id>", "version": 1, "hash": "<base64url>"}}}
```

Clients check five things:

- the signature;
- that the author device was active and held a writer or admin grant at `log_head`;
- the chain from the last version they saw, which they pin locally per store and reject going backwards;
- the epoch's key commitments;
- each fetched object's hash against its entry.

### Lookup object

The lookup object holds, per secret name, the lines of the secret that carry its lookup fields and the browser policy fields `pasejo-fill` and `pasejo-confirm`: by default the URL keys `uri url link site` and the username keys `username user login email`. The password line, the first non-blank line, is never included; a reader that parses a lookup text as a secret puts a placeholder password line above it. It is encrypted under `D_e`, and its hash sits in the index. The browser host's query phase reads it instead of every secret, so a query is audited as one lookup and a fill as one object fetch. The lookup keys are store settings kept in the encrypted index:

```json
{"settings": {"uri_keys": ["uri", "url", "link", "site"], "username_keys": ["username", "user", "login", "email"]}}
```

## Membership log

The membership log is append-only and kept per organization. Each entry is an envelope with context `log-entry/v1` whose payload is:

```json
{"org": "<id>", "seq": 12, "prev": "<base64url hash of entry 11, null for 0>", "time": 1763251200000,
 "author": "<device id>", "kind": "store.grant", "body": {}}
```

| Kind | Body | Valid authors |
|---|---|---|
| `org.genesis` (seq 0 only) | `name`; `founder` user; `device` (founding device); `settings` | the founding device |
| `org.settings` | `settings`: `device_approval` (`manual` or `automatic`), `two_admin` (bool), `auto_join` (bool) | an admin; plus a second admin when `two_admin` |
| `member.role` | `user`, `role`: `admin`, `approver`, `auditor`, `member` or `none` | an admin; plus a second admin when `two_admin` and the change grants or removes `admin` |
| `device.add` | `device`: `id`, `user`, `kind`, `recipient`, `signing_key`, `label` | an admin or approver, or an active device of the same user |
| `device.revoke` | `device` | an admin, or an active device of the same user |
| `recovery.set` | `recipient` (age) or `null` | an admin; plus a second admin when `two_admin` |
| `team.create` | `team`, `name` | an admin |
| `team.member` | `team`, `user`, `member` (bool) | an admin |
| `team.delete` | `team` | an admin |
| `store.create` | `store` | any member with an active device; the author becomes the store's admin |
| `store.grant` | `store`, `principal` (`{"user": id}` or `{"team": id}`), `role`: `reader`, `writer`, `admin` or `none`, optional `expires` | a store admin |
| `store.epoch` | `store`, `epoch`, `names_commit`, `data_commit`, `full` (device ids), `names_only` (device ids) | a store admin |
| `store.delete` | `store` | a store admin; plus a second admin when `two_admin` |

Further rules:

- **Devices.** A user's first device must be added by an admin or approver. A device added by another device of the same user inherits nothing beyond that user's own rights.
- **Removal.** `member.role` with `none` removes the user from the organization and implicitly revokes the user's devices. Revoked devices never return; re-enrolling generates new keys.
- **Epochs.** Epoch numbers start at 0, right after `store.create`, and increase by one.
- **Effective grants.** A grant with `expires` stops being effective at that time, both for the server's enforcement and for the clients' wrapping set.
- **Machines.** Machine users are ordinary users whose server record marks them as machines. Their devices follow the same rules.

Every client and the server replay the log with the same code from `pasejo-protocol`, which checks:

- the chain;
- each signature;
- each author's rights at that point.

The server refuses entries that fail replay, and entries whose `time` lies more than five minutes from its clock. Replay judges grants and expiries at the time an entry names, so a backdated entry could otherwise act on a grant that has already ended. Two-admin entries first exist as proposals. A second admin co-signs the proposal's payload, and the server appends it only once replay accepts both signatures.

**Split views.** A server could show different logs to different clients. Every index version and every log entry names its author's log head. A client rejects an index or entry whose `log_head` is not part of its own verified chain, so two diverging views are detected the first time they interact. The organization's co-signature on audit checkpoints, which includes the log head, extends the check to auditors.

**Pinning.** `pasejo org join` enrolls the device, then fetches the log and shows the fingerprint of its genesis entry, the first 20 bytes of the entry's hash grouped like a device fingerprint, for comparison over another channel, or takes it as `--fingerprint`. Only members and enrolling devices may read the log, so enrollment comes first; a device whose comparison fails never registers the organization, and its enrollment stays unused. Clients store the genesis hash, and every later replay must start from it.

## Device enrollment

1. **Key generation.** The client generates its device keys and signs in. The session is bound to the signing key at once, and the device starts out pending.
2. **Pending.** A pending device may read the membership log and its own enrollment status, and nothing else.
3. **Approval.** In `manual` mode, an admin or approver approves the device in the CLI or the console. In strict comparison, the approver compares the device fingerprint shown on both screens before signing `device.add`. A user who already has an active device can approve further devices from it. In `automatic` mode, an admin machine device running `pasejo org actions --run` approves pending devices of organization members automatically. A compromised server or identity provider could then enroll a device of its own. This is the same trust that any server holding keys would receive, so `automatic` is a separate setting and `manual` is the default.

## Authentication

### People

- **CLI.** The CLI signs in with the OAuth 2.0 device authorization grant (RFC 8628) against the server:
  1. `POST /v1/auth/device` registers the device's public keys and returns a user code.
  2. The user opens `/auth/verify` in a browser, signs in through the organization's OIDC provider, sees the device label and fingerprint, and approves.
  3. The CLI polls `POST /v1/auth/token` with an envelope (context `token/v1`) over the device code, signed by its signing key, and receives a session token bound to that key.
- **OIDC.** The server is an OIDC relying party using the authorization code flow with PKCE (RFC 7636), `state` and `nonce`. It validates the ID token's issuer, audience, expiry and nonce against the provider's discovery document and JWKS. Users are identified by issuer and subject.
- **Console.** The console uses the same OIDC flow. The resulting cookie is `HttpOnly`, `Secure` and `SameSite=Strict`, and it serves one purpose: exchanging it, together with a `token/v1` envelope signed by the browser device key, for a session token bound to that key.

### Machines

Machines present a JWT from a configured workload issuer, such as a Kubernetes service-account token with the audience `pasejo-server` or a GitHub Actions OIDC token. They do so at `POST /v1/auth/workload`, together with a `token/v1` envelope. The server validates the token against the issuer's JWKS, fetched from a URL or read from a file, which serves private clusters. It then maps issuer and subject to a machine user through rules in its configuration.

### Sessions

- **Lifetimes.** Session tokens are 32 random bytes, stored as SHA-256 hashes. They live 12 hours for people and machines, 15 minutes for break-glass, and 1 hour for emergency sessions, which also end when the emergency window closes. Signing out revokes the session.
- **Device binding.** A session is bound to one signing key. Revoking the device or deactivating the user ends its sessions immediately.

### Request proofs

Every authenticated request carries four headers:

- `Authorization: Pasejo <token>`
- `Pasejo-Timestamp`
- `Pasejo-Nonce` (16 random bytes)
- `Pasejo-Proof`

The proof is a `request/v1` signature over:

```text
METHOD "\n" path-and-query "\n" timestamp "\n" nonce "\n" base64url(SHA-256(body)) "\n" base64url(SHA-256(token))
```

The server refuses:

- timestamps more than 60 seconds away from its clock;
- nonces it has seen within the window;
- proofs that do not verify against the session's key.

A token copied off a machine is useless without the device key.

### Provisioning

Each organization can create SCIM bearer tokens, which are shown once and stored hashed. SCIM 2.0 (RFC 7643, RFC 7644) serves `/scim/v2/{org}/Users` and `/scim/v2/{org}/Groups`, plus the discovery endpoints. Supported operations are list with `eq` filters on `userName`, `externalId` and `displayName`, get, create, replace, patch (`active`, names, emails, group members) and delete.

- **Linking.** A SCIM user links to the account with the same email, or to the OIDC identity whose verified email matches on first sign-in.
- **Groups.** SCIM groups become teams. A team takes a group member once that user belongs to the organization.
- **Deactivation.** Deactivating or deleting a SCIM user suspends them in that organization at once, before any log entry removes them.
- **Scope.** A SCIM client acts on its own organization only. It never renames the account, ends its sessions or affects its other organizations, so one organization's identity provider cannot reach users it shares with another.
- **Log changes.** Changes that need log entries surface as pending actions.

## Pending actions

The server signs nothing, so changes that need a signature surface as pending actions. `GET /v1/orgs/{org}/actions` computes them on every request from the difference between the desired state and the log, so no action queue can go stale:

| Action | Desired state comes from | Resolved by |
|---|---|---|
| `approve-device` | pending enrollment requests | `device.add` |
| `join-member` | SCIM or `auto_join` sign-ins | `member.role member` |
| `remove-member` | SCIM deactivation or deletion | `member.role none` |
| `sync-team` | SCIM group membership | `team.create`, `team.member` |
| `rotate-epoch` | a device in the current epoch's `full` or `names_only` set that is no longer entitled to it (revoked, removed, expired grant) | new `store.epoch` and wrappings |
| `wrap-keys` | an entitled device without a wrapping for the current epoch | upload of wrappings |
| `revoke-expired` | grants past `expires` | `store.grant none` |
| `revoke-grant` | `revoke` decisions of access reviews | `store.grant none` |
| `cosign-checkpoint` | audit checkpoints the organization has not co-signed | co-signature |

`pasejo org actions --run` executes every action its device has the rights and keys for. An organization that wants SCIM and automatic approval to take effect without a person runs it on a schedule as an admin machine. The console executes the same actions after a click.

## Sync protocol

All endpoints are under `/v1`, take and return JSON, and require a session with a request proof unless marked otherwise.

| Endpoint | Purpose |
|---|---|
| `GET /server` (no auth) | server version, the server's checkpoint public key, and the OIDC sign-in URL |
| `POST /auth/device`, `POST /auth/token`, `POST /auth/web`, `POST /auth/workload`, `POST /auth/emergency`, `POST /auth/logout` | sessions |
| `GET /me` | the session's user and the organizations they belong to |
| `POST /orgs` | create an organization from a genesis envelope |
| `GET /orgs/{org}/log?since={seq}` | log entries after `seq` |
| `POST /orgs/{org}/log` | append an entry |
| `POST /orgs/{org}/log/proposals`, `GET /orgs/{org}/log/proposals`, `POST /orgs/{org}/log/proposals/{id}/cosign` | two-admin entries |
| `POST /orgs/{org}/enrollments`, `GET /orgs/{org}/enrollments`, `GET /orgs/{org}/enrollments/{device}` | enroll a device, list pending devices, and a device's own status |
| `GET /orgs/{org}/users` | names and emails of the organization's members |
| `GET /orgs/{org}/actions` | pending actions |
| `GET /orgs/{org}/stores` | stores the session may see, with their current epoch and index version |
| `GET /orgs/{org}/stores/{store}/keys` | this device's wrappings and the epoch chain objects |
| `PUT /orgs/{org}/stores/{store}/epochs/{epoch}` | upload `prev_names`, `prev_data` and wrappings for a new epoch |
| `POST /orgs/{org}/stores/{store}/wrappings` | upload wrappings for the current epoch |
| `GET /orgs/{org}/stores/{store}/index` | current index envelope; `If-None-Match` with the version |
| `GET /orgs/{org}/stores/{store}/lookup` | current lookup object |
| `POST /orgs/{org}/stores/{store}/objects` | fetch objects by identifier and version; one audit event each |
| `POST /orgs/{org}/stores/{store}/commits` | commit an index version with its objects |
| `GET /orgs/{org}/stores/{store}/rotation?user={id}` | secrets the user could read or did read since their access ended, and whether each was rewritten since |
| `GET /orgs/{org}/recovery` | every store's recovery wrappings, epoch chain, current index and current objects; admins only |
| `GET /orgs/{org}/audit`, `GET /orgs/{org}/audit/checkpoints`, `POST /orgs/{org}/audit/checkpoints/{seq}/cosign` | audit |
| `POST /orgs/{org}/reviews`, `GET /orgs/{org}/reviews`, `POST /orgs/{org}/reviews/{id}/decisions` | access reviews |
| `POST /orgs/{org}/scim-tokens`, `GET /orgs/{org}/scim-tokens`, `DELETE /orgs/{org}/scim-tokens/{id}` | SCIM tokens |
| `GET /audit` | the server's own audit chain, for break-glass sessions |
| `POST /break-glass/login` (no session), `POST /break-glass/emergency`, `GET /break-glass/users`, `GET /break-glass/orgs`, `POST /break-glass/users/{id}/suspend`, `POST /break-glass/users/{id}/reactivate`, `POST /break-glass/users/{id}/sessions/revoke` | break-glass |

**Commits.** A commit is the next index envelope plus its new objects, applied atomically. The server checks:

- the signature;
- that the author device is active and the user holds an effective writer or admin grant;
- that `version` is the current version plus one;
- that `prev` matches the current version's hash;
- that `epoch` is the current epoch;
- that `changed` equals the identifiers of the uploaded objects, and that no identifier is both changed and deleted;
- that each object carries the index's epoch and a non-empty ciphertext of at most 1 MiB;
- that each object's version exceeds the stored one;
- that the lookup object matches `lookup_hash`;
- that `time` lies within five minutes of the server's clock.

Commit bodies may reach 32 MiB; every other request body is capped at 1 MiB. The store's version moves with a compare-and-set inside the commit's transaction, so of two concurrent commits to one version exactly one succeeds.

A commit against an outdated version gets `409` with the current version. The client fetches the new index, reapplies its changes and retries. It reports a conflict only when both sides changed the same name, using the rules of `cli/merger.rs`.

A log entry whose sequence number or predecessor is no longer the head gets `409` with code `conflict` as well; the client fetches the entries it missed, rebuilds the entry on the new head and retries. Every other rejection by log replay gets `400` with code `rejected`.

**History.** Object versions are immutable, and the server keeps each object's last ten versions. Showing an HOTP code is a read followed by a commit.

**Client cache.** Clients keep the latest index, the wrappings and the pinned log head in the pasejo data directory, all in encrypted form, so listing a store works from the cache. Secret objects are always fetched, so every read of a value reaches the audit log.

**Errors.** Every error is a JSON body with a stable `code`, such as `conflict`, `forbidden` or `unknown_store`. The CLI translates these codes into its eight languages.

## Revocation and rotation

Removing a user, revoking a device, ending a grant or reaching an expiry runs in three steps:

1. **Access stops.** The server stops serving the store to that user or device at once.
2. **New epoch.** The next `rotate-epoch` action, run by a store admin's client or an admin machine:
   1. generates fresh keys;
   2. appends `store.epoch` with the new commitments and recipient sets;
   3. uploads the chain objects and wrappings;
   4. re-encrypts the index with the next commit.

   A device counts as holding an epoch's keys when the `store.epoch` entry names it or when it received a wrapping of that epoch later, so a reader added mid-epoch triggers a rotation when their access ends.

   Secret objects are re-encrypted when they are next written, or all at once with `pasejo store rekey`. Lazy re-encryption gives up nothing against the removed party, because everything still under the old epoch was readable by them before removal.
3. **Rotation list.** The rotation endpoint returns every identifier that existed while the party had access and every identifier they fetched, marking those rewritten since. `pasejo store rotation` and the console show the names. Rotating the values is the revocation that counts, because the removed party may have kept plaintext.

## Access reviews

An organization admin or store admin opens a review for a store, naming reviewers and a due date. Each reviewer marks every grant `keep` or `revoke`. Revoke decisions surface as `revoke-grant` actions for organization admins and the store's admins, whose clients resolve them with `store.grant none`. A review is complete when every grant has a decision. A reviewer never decides on a grant they benefit from, held directly or through a team, so somebody else must confirm each reviewer's own access. Reviews and decisions are stored server-side and audited.

## Audit

### Events

The server records:

- authentication attempts and their results, including break-glass logins;
- log appends, proposals and co-signatures;
- key, index and lookup fetches;
- object fetches per identifier and version;
- commits per changed or deleted identifier;
- audit queries and exports;
- reviews;
- SCIM changes;
- emergency windows;
- every refused request.

Each event carries a sequence number, time, organization, actor user, actor device, session kind, client IP address, user agent, action, store, identifier, version, outcome and details.

### Integrity

- **Append-only storage.** The events table only ever receives inserts. Each event stores the hash of its predecessor and its own hash, which is SHA-256 over the predecessor's hash and the event's serialized fields.
- **Checkpoints.** Every 1,000 events, and at least hourly when events occurred, the server signs a checkpoint with its Ed25519 key. A checkpoint names the organization, the sequence number, the event hash, the membership log head and the time. The server's own chain, which records events outside any organization such as sign-ins, break-glass use and refusals for unknown organizations, has checkpoints without an organization and without a log head.
- **Co-signatures.** Organization devices may co-sign checkpoints, and `pasejo org actions --run` co-signs every checkpoint whose log head the device verified. A server that later rewrites events or presents a forked log then contradicts a signature it cannot produce.
- **Verification.** `pasejo audit verify` recomputes the chain from the last checkpoint the client trusts and checks the server's and the organization's signatures. The server's checkpoint key is pinned at the device's first sign-in to the server, and a later sign-in that sees another key fails.

### Names

Events record identifiers. `pasejo audit log` and the console resolve them with the reader's names keys for every epoch. An auditor sees "alice read `prod/database` in store `production` at 10:02" without being able to read the value.

### Export and retention

- **Pull.** The API pages through events by sequence number.
- **Stdout sink.** It writes each event as a JSON line for log collectors.
- **Webhook sink.** It posts batches signed with HMAC-SHA256, retries with backoff and keeps its position in the database.
- **Retention.** Retention is set in days. Pruning removes events older than that but keeps the last checkpoint before the cut, so the chain stays verifiable.

### Limits

Compliance documentation has to state:

- plaintext copied after decryption;
- reads the bridge serves from its memory within one request;
- the bridge's attribution, which names the workload's tenant label and not the ExternalSecret.

## Break-glass access

Break-glass covers two failures: SSO is unavailable, and every key holder of an organization is gone. Two separate mechanisms handle them, because no single credential should be able to do both.

### Server break-glass account

- **Creation.** The account is created only on the server host with `pasejo-server break-glass create`, which needs access to the server's configuration and database. It generates a 32-character password and a TOTP secret, prints both once, and stores the password as an Argon2id hash (64 MiB, 3 iterations, 1 lane). The TOTP secret is stored encrypted under a key derived from the server's master key file.
- **Disabled by default.** It must be enabled in the configuration.
- **Login.** Login needs the username, the password and a current TOTP code, and is accepted only from configured networks (default: loopback). After five failures the account is locked for 15 minutes, and every attempt is audited. Each success is sent to the audit webhook as an alert.
- **Sessions.** A session lasts 15 minutes and is bound to a signing key the client generates for it.
- **Powers.** It can list organizations and users, suspend or reactivate users, end sessions, read the audit log, and open an emergency sign-in window of at most 24 hours. It cannot sign log entries and holds no wrapping, so it can neither grant access nor read a secret or a name.

### Emergency sign-in window

While the window is open, devices that are active in some organization's log can obtain a session of at most one hour, never outlasting the window, at `POST /v1/auth/emergency` by signing a `token/v1` envelope with their device key, without SSO. Closing the window early ends every emergency session. Opening, using and closing the window is audited. The window lets an organization keep working while its identity provider is down, without creating any new trust.

### Organization recovery recipient

`recovery.set` names an age recipient whose identity the organization keeps offline, for example printed and split among officers. Every store's full wrapping set includes it, under the device identifier `recovery_id`, the first 16 bytes of `SHA-256("pasejo recovery v1" ‖ 0x00 ‖ recipient)`. `pasejo org recover --identity FILE --output DIR` fetches `GET /orgs/{org}/recovery`, which returns, for every store, the recovery recipient's wrappings, the epoch chain, the current index and the current version of every object. It decrypts each store with the recovery identity, checks keys, index and objects as a device would, and writes each one as a file store encrypted to the recovery recipient. The export is for admins, and each attempt is audited as `recovery.export`. Break-glass sessions read no organization data, the log included, and the log is what the export is verified against; when every admin device is lost, the emergency window admits an existing device again.

## Kubernetes

The bridge (`pasejo-bridge`) runs in the cluster as a machine device.

- **Keys.** `pasejo-bridge keygen` writes the device keys, which the operator stores as a Kubernetes Secret and mounts read-only.
- **Enrollment.** On start, the bridge signs in with its projected service-account token (audience `pasejo-server`) and requests enrollment if its device is not active yet. An approver approves it like any device.
- **API.** It serves `GET /v1/secrets/{store}/{name}` with an optional `line` or `field` query parameter. Here `{store}` is a store label or identifier, and `{name}` is a secret name with `/` allowed. The reply is `{"value": "..."}`.
- **Bridge credentials.** Bridge credentials map bearer tokens to tenant labels. Each read is audited server-side as the machine with that tenant label.

The external-secrets.io documentation, as read on 2026-11-15 (the page footer says © 2025), states the following about the Generic Webhook provider:

- Its stability level is alpha.
- Templating covers `url`, `headers`, `body` and `result.jsonPath`, and `.remoteRef.key` is available. A templated host fails store validation.
- Credentials come from Secrets labeled `external-secrets.io/type: webhook`.
- Templates receive no metadata about the ExternalSecret that triggered the request.
- An HTTP 404 means the secret was deleted, which triggers the `deletionPolicy`.

These facts shape the bridge:

- **404.** It answers 404 only for a name absent from an index it fetched and verified.
- **503.** It answers 503 whenever the server, the index or its keys are unavailable, so an outage never deletes Secrets in the cluster.
- **Attribution.** ESO does not say which ExternalSecret asked. One SecretStore per namespace, each with its own bridge credential, gives every read a namespace label.

## Browser console

The server serves the console under `/console/` as static HTML, CSS and JavaScript modules, with no build step and no third-party code.

- **Headers.** Every console response carries:
  - a Content-Security-Policy of `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'`;
  - `Cross-Origin-Opener-Policy: same-origin`;
  - `Referrer-Policy: no-referrer`.
- **Device keys.** On first sign-in, the console generates the browser's device keys, keeps them non-extractable in IndexedDB, and enrolls the browser as an `hpke` device. Another device of the same user, or an approver, approves it.
- **Capabilities.** With names-only wrappings, the console shows:
  - organization members, roles, teams and devices;
  - store labels and grants;
  - pending actions, and it executes them;
  - the audit log with resolved names;
  - access reviews;
  - rotation lists.

  It signs log entries with the browser's key. It never sees a data key.
- **Language.** The console follows the browser's language preference among the CLI's eight languages, so it needs no language switcher.
- **Break-glass.** A separate break-glass page offers the login and the emergency window.

## Client changes

- **Registration.** `StoreRegistration` gains a `remote` table: server URL, organization and store identifier. The file path becomes the store's cache directory.
- **Fetch plan.** Commands declare what they read, through a fetch plan given to `with_store`:
  - names only: `secret list`, `secret remove`, `otp list`, `otp remove`;
  - named entries: `secret show`, `secret add`, `secret generate`, `secret edit`, `secret copy`, `secret move`, the `otp` commands that read or write one entry, `secret resolve` and `exec`;
  - the lookup object: `browser query`;
  - everything: `secret grep`, `secret audit`, `export`, `store decrypt`.

  For a remote store, the funnel fetches that much into a `PasswordStore`, runs the command's closure, and turns the difference into a commit. A removed name whose value reappears under a new name becomes a rename that keeps the identifier. File stores ignore the plan.
- **Existing interfaces.** `secret resolve`, `exec`, the fnox plugin, breto and the browser host keep their interfaces. The browser host reads the lookup object for a query and one object for a reveal.
- **New commands:**

| Command | Purpose |
|---|---|
| `pasejo server login URL [--label L] [--recipient R] [--workload-token FILE \| --emergency]`, `pasejo server logout [URL] [--forget]`, `pasejo server status` | sessions |
| `pasejo org create NAME`, `pasejo org join URL ORG [--fingerprint F]`, `pasejo org stores` | organizations |
| `pasejo org devices`, `pasejo org approve DEVICE [--fingerprint F]`, `pasejo org revoke DEVICE` | devices |
| `pasejo org role USER ROLE`, `pasejo org settings ...`, `pasejo org recovery set RECIPIENT`, `pasejo org recover --identity FILE --output DIR` | administration |
| `pasejo org proposals`, `pasejo org cosign PROPOSAL` | two-admin changes |
| `pasejo org review open --reviewer EMAIL... --due DATE [--store S]`, `pasejo org review list`, `pasejo org review decide REVIEW PRINCIPAL keep\|revoke` | access reviews |
| `pasejo org scim-token create`, `pasejo org scim-token list`, `pasejo org scim-token revoke TOKEN` | SCIM tokens |
| `pasejo org team create NAME`, `pasejo org team add TEAM USER`, `pasejo org team remove TEAM USER` | teams |
| `pasejo org actions [--run]` | pending actions |
| `pasejo store create-remote LABEL [--org ORG]`, `pasejo store add-remote STORE --name NAME`, `pasejo store grant PRINCIPAL ROLE [--expires DATE]`, `pasejo store rekey`, `pasejo store rotation USER`, `pasejo store migrate --to-remote ORG` | remote stores |
| `pasejo audit log [--store S] [--since SEQ]`, `pasejo audit verify` | audit |

Commands taking one organization accept `--org` with a name or identifier and default to the only registered one. A change the log rejects for lack of a second admin is proposed instead, and the proposal's identifier is printed. `store grant` runs its store's key wrapping and rotation actions itself.

- **Inapplicable commands.** `pasejo recipient`, the hook commands and `store merge` report that they do not apply to remote stores.
- **HTTP.** The CLI talks HTTP through `ureq`, which it already depends on, so it needs no async runtime.

## Server

- **Stack.** Rust with tokio and axum, and sqlx with the `Any` driver on SQLite or PostgreSQL. SQL is written once with `$n` placeholders, rewritten to `?n` for SQLite, and each backend has its own migrations. Objects live in the database.
- **State.** Server processes are stateless apart from caches, and each refreshes its replayed membership-log state when the stored head moves. High availability comes from PostgreSQL.
- **Configuration.** One TOML file, whose path also comes from `PASEJO_SERVER_CONFIG`:
  - listen address and public URL;
  - database URL;
  - master key file, a 32-byte secret from which the checkpoint signing key and the TOTP encryption key are derived;
  - optional TLS certificate and key;
  - OIDC issuer, client id and client secret file;
  - workload issuers and rules;
  - break-glass settings;
  - audit checkpoint, retention and sink settings;
  - organization creation policy;
  - trusted proxies for client addresses;
  - an optional separate metrics listen address.
- **Hardening:**
  - per-address rate limits on authentication endpoints;
  - request bodies capped at 1 MiB, commits at 32 MiB;
  - `Cache-Control: no-store` on every API response;
  - HSTS when the public URL uses HTTPS;
  - structured logs that never contain tokens, ciphertexts or proofs.
- **Operations.** `/healthz` and `/readyz` for probes. `/metrics`, in the Prometheus text format with request, refusal, audit, commit and object-read counters, is served only on the separate `metrics_listen` address, so an ingress that routes the main listener never exposes it.
- **Transactions.** SQLite transactions begin with `BEGIN IMMEDIATE`, taking the write lock up front; a deferred transaction that reads before writing fails outright when another writer commits in between. PostgreSQL locks the rows it updates.
- **Server commands:** `pasejo-server serve`, `pasejo-server migrate`, `pasejo-server keygen`, `pasejo-server break-glass create|disable`, and `pasejo-server audit prune`.

## Repository layout and releases

- **Workspace.** The CLI stays the root package of the Cargo workspace. `cargo test --workspace` runs every crate's tests, and CI also runs:
  - the server tests against PostgreSQL;
  - the console's JavaScript tests under Node with its WebCrypto;
  - cross-language test vectors generated by `pasejo-protocol` and checked by the console tests.
- **CLI release.** The CLI release publishes `pasejo-protocol`, `pasejo-client` and `pasejo` to crates.io in dependency order, and counts changes under `src/`, `crates/pasejo-protocol/` and `crates/pasejo-client/`.
- **Server release.** A separate release workflow builds `pasejo-server` and `pasejo-bridge` for Linux on x86_64 and aarch64 and publishes the binaries and container images to `ghcr.io/metio`. Images are built from `docker.io/library/rust` with distroless runtime images and signed with cosign keyless. The version format and cadence follow the CLI.
