<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
-->

# pasejo server design

Status: proposal, 2026-11-15. Nothing described here is implemented yet.

## Summary

`pasejo-server` is a self-hosted server through which an organization manages its stores. It adds:

- SSO sign-in;
- teams and roles;
- syncing between people and machines;
- delivery of secrets to Kubernetes;
- an audit log that compliance staff can query.

The server never holds a key that decrypts a secret. Clients encrypt secret names, values, fields and URLs with keys the server cannot read. The server sees:

- the organization's structure: users, teams, devices, store labels and grants;
- opaque secret identifiers and padded sizes;
- which device fetched or wrote which identifier, and when.

Five decisions shape the design:

1. **Remote stores are a second backend.** Remote stores are a second kind of store next to today's file stores. File stores keep their single-file format, their hooks and their git workflow unchanged.
2. **Encrypted index.** A remote store holds one encrypted object per secret under a random identifier, plus an encrypted index that maps names to identifiers. The index hides names, which the single file does for a file store today. It also makes per-secret sync and per-secret audit possible.
3. **Epoch keys.** Each store has epoch keys, wrapped with age to every authorized device, and secret objects are encrypted under those keys. A YubiKey user therefore touches the key once per command, as today. Adding a member wraps two keys instead of re-encrypting every secret.
4. **Signed membership log.** A signed membership log decides who may decrypt, and clients verify it. The server enforces access, but it cannot add a recipient: clients encrypt only to devices whose enrollment carries a valid signature from an admin or from another device of the same user.
5. **Hash-chained audit.** Every fetch and every commit lands in a hash-chained audit log. Auditors hold a key that decrypts names only, so they resolve identifiers to names on their own machine without being able to read values.

## Goals and non-goals

**Goals:**

- OIDC sign-in, organizations, teams and roles;
- SCIM provisioning;
- sync for people and machines;
- audit of every write and of every read the server observes;
- a Kubernetes bridge for external-secrets-operator;
- revocation that tracks which secrets need rotating;
- a deployment of one binary plus PostgreSQL.

**Non-goals:**

- server-side decryption;
- replacing git sync for file stores;
- enforcing anything on a client after it has decrypted a value.

A hosted multi-tenant service is out of scope, although the data model keeps an organization column everywhere so one installation can serve several organizations.

## What the server learns

A remote store reveals more metadata than a file store synced through git. The git host of a file store sees one file, its size, commit times and committers. It cannot tell which secret changed or how many there are. A remote store reveals the number of secrets, their padded sizes, and which identifier each device reads and writes. That is the price of per-secret audit and per-secret sync. Organizations that do not want to pay it keep file stores.

| Data | Visible to the server |
|---|---|
| Users (OIDC issuer and subject, email, display name), teams, devices and their public keys | yes: authorization and SSO need them |
| Store labels | yes: labels are chosen by the organization, which can pick neutral ones |
| Grants: who holds which role on which store | yes |
| Number of secrets and OTP entries per store | yes: one object each |
| Size of each secret | padded with Padmé, which leaks O(log log L) bits of a length L |
| Secret names, field names, URLs, values, OTP seeds | no |
| Which identifier a device fetched or wrote, and when | yes |
| Renames | no: a rename changes only the encrypted index |

Access patterns are the main residual leak. Without learning a name, the server can correlate the fetch of one identifier with outside events, such as a deployment that follows it every time.

## Name hiding

The encrypted index with random identifiers is the chosen mapping:

- Identifiers carry no information about names.
- A rename touches only the index.
- Listing secrets reads one object.

The alternatives considered:

- **Identifiers derived from names**, as an HMAC of the name under a store key. A client can locate an object without the index. A removed member who kept the key can test guessed names against identifiers indefinitely, which confirms that `prod/database` exists. Renames change identifiers, and listing still needs an index. Rejected.
- **The whole store as one remote object**, which is today's model on a server. It hides counts and access patterns, but the server can audit only "fetched the store". Every read becomes a read of everything, and every concurrent write conflicts. File stores with a hook that uploads the file already offer this.
- **Oblivious access (ORAM, PIR).** It hides access patterns too, at a cost in bandwidth and complexity out of proportion to secrets of a few hundred bytes. Rejected.

## Keys and objects

### Device keys

Each device, whether a laptop, a CI runner or an in-cluster bridge, holds:

- **an age recipient and its identity.** Any recipient pasejo accepts today works: X25519, SSH, or an age plugin such as a YubiKey. Hardware keys keep working.
- **an Ed25519 signing key** that pasejo generates. It signs membership log entries, commits and request proofs.

The age crate already brings `chacha20poly1305`, `hkdf` and `sha2` into the CLI's build (checked with `cargo tree -i`). `ed25519-dalek` is in `Cargo.lock` only through the `ssh-key` dev-dependency, so signing adds it as a regular dependency.

### Store epochs

Each store has a sequence of epochs, and epoch *e* has two random 256-bit keys:

- **the names key `N_e`** decrypts the index;
- **the data key `D_e`** decrypts secret objects and the lookup object.

For each device with access, the server stores a wrapping: an age file encrypted to the device's recipient, holding the store identifier, *e*, `N_e` and `D_e`. An auditor's wrapping omits `D_e`. Wrapping per device keeps enrollment cheap, since a new device adds one wrapping, and each device downloads only its own.

Epochs are chained backwards. Epoch *e* stores `N_{e-1}` and `D_{e-1}` encrypted under its own keys, so a holder of epoch *e* reads objects that still carry an older epoch. A removed member holding *e − 1* learns nothing about *e*.

Readers and writers hold the same symmetric keys. Signatures, which clients verify against the membership log, are what separate a writer from a reader.

### Secret objects

| Part | Content |
|---|---|
| Identifier | random 128 bits |
| Version | counter per identifier |
| Plaintext | kind (`secret` or `otp`) and content: the secret text, or the OTP parameters including the HOTP counter; padded with Padmé |
| Cipher | XChaCha20-Poly1305 under `HKDF-SHA256(D_e, salt = store id, info = "pasejo secret v1" ‖ identifier)`, random 192-bit nonce |
| Associated data | format version, store identifier, epoch, identifier, version |

Binding the identifier and version into the associated data stops the server from swapping two ciphertexts or serving an object under another identifier.

### Index

The index maps each name to its identifier, kind, version and ciphertext hash. It covers secrets and OTP entries, the two maps `PasswordStore` keeps today. It is encrypted under a key derived from `N_e`, with the store identifier, epoch and index version as associated data, and padded.

Each index version is signed by the committing device and names the hash of its predecessor, which forms a chain. Clients pin the last version they saw and reject an older or a forked one. The ciphertext hash in each entry lets a client reject a stale or substituted object.

### Lookup object

The lookup object holds, per name, the values of the lookup fields: by default the URL keys `uri url link site` and the username keys. It is encrypted under `D_e`.

The browser host's query phase reads the lookup object instead of every secret. A query is then audited as one lookup, and a fill fetches the one secret it reveals. This maps onto the two-phase query and reveal the host already implements. Every member must build the same object, so for remote stores the lookup keys become a store setting instead of a per-command `--uri-keys`.

## Membership log

The membership log is append-only and kept per organization. Every entry is signed by a device and carries the hash of its predecessor.

| Entry | Content | Valid signers |
|---|---|---|
| `org.genesis` | organization id, founding user and device | the founding device; clients pin its hash |
| `device.add` | user, device id, age recipient, signing key | an org admin or device approver, or an existing device of the same user |
| `device.revoke` | device id | an org admin or a device of the same user |
| `org.role` | user and role: admin, device approver, auditor, member | an org admin; granting admin takes two admins when the organization asks for it |
| `team.member` | team, user, added or removed | an org admin, the team owner, or the organization's automation identity |
| `store.create`, `store.delete` | store id, label | an org admin; deletion takes two admins when the organization asks for it |
| `store.grant`, `store.revoke` | store, principal (user, team or machine), role (reader, writer, admin, auditor), optional expiry | a store admin |
| `store.epoch` | store, epoch, hash of the set of devices it was wrapped to | a store admin |

A client derives a store's authorized devices by replaying the log, and encrypts only to them. The server mirrors the log into tables for enforcement and rejects entries whose signature does not verify. When the server holds a grant the log lacks, clients ignore that grant.

**Split views.** A server could show different logs to different clients. Each commit and each log entry embeds the author's current log head. A client rejects a commit that references a head outside its own verified chain, so two diverging views are detected the first time they interact. External witnesses that co-sign log heads would close the gap before that interaction and are left for later.

**Pinning.** `pasejo org join <url>` shows the genesis fingerprint for comparison over another channel, or takes it as `--fingerprint`. Without either, it trusts the first log it sees.

## Device enrollment

- **The first device of a new user.** The user signs in through SSO. The client generates its keys and uploads an enrollment request, and a device approver signs `device.add`. In strict mode, both screens show a short code derived from the device keys for the approver to compare. In lenient mode, the organization's automation identity approves SSO-authenticated requests automatically. A compromised server or identity provider can then enroll a device of its own, which is the trust a Vault-style server receives anyway. The mode is an organization setting.
- **Further devices.** An existing device of the same user approves the new one by scanning a code.
- **Machines.** Machines enroll as described under [Kubernetes](#kubernetes) and [Authentication](#authentication).

## Sync protocol

The protocol is HTTPS with JSON bodies; binary objects travel base64-encoded. Version 1 endpoints:

| Endpoint | Purpose |
|---|---|
| `GET /v1/orgs/{org}/log?since={seq}` | log entries after the client's pinned head |
| `POST /v1/orgs/{org}/log` | append a signed entry |
| `GET /v1/stores/{store}/keys` | this device's wrappings |
| `GET /v1/stores/{store}/index` | current index; `ETag` is the index version |
| `GET /v1/stores/{store}/lookup` | current lookup object |
| `POST /v1/stores/{store}/objects:get` | fetch objects by identifier; one audit event each |
| `POST /v1/stores/{store}/commits` | atomic commit, conditional on a base index version |
| `GET /v1/orgs/{org}/audit` | audit events for auditors |
| `/scim/v2/Users`, `/scim/v2/Groups` | SCIM provisioning |

A commit carries:

- the base index version;
- the new index, and a new lookup object when it changed;
- the new objects;
- the identifiers it changed and deleted, in clear;
- the author's signature over all of it.

The clear identifiers let the server audit and collect garbage. Auditors' tools verify them against the decrypted index diff.

**Conflicts.** A commit against an outdated base returns `412`. The client then fetches the new index, reapplies its change and retries. It reports a conflict only when both sides changed the same name, using the rules of `cli/merger.rs`:

- a value both sides agree on is kept;
- a change made on one side wins;
- a change made on both sides is a conflict.

Showing an HOTP code is a read followed by a commit.

**History.** Objects are immutable per identifier and version, and the server keeps old versions according to its retention policy. A restore is a new commit that re-encrypts the old value.

**Client cache.** A client may cache ciphertext only, in the pasejo data directory. A store policy chooses between:

- `none`: every read fetches, so every read is audited;
- `ciphertext`: offline reads are possible, and reads served from the cache are not audited.

Clients honor the policy, but nothing enforces it on a modified client.

**Hooks.** Remote stores have no pull or push hooks; the protocol is the sync.

## Authentication

**People** sign in with OIDC, with the server as the relying party:

- The CLI uses the device authorization grant (RFC 8628) by default, and a loopback redirect with PKCE (RFC 8252, RFC 7636) when it can open a browser.
- SAML-only organizations go through an OIDC broker such as Dex or Keycloak instead of SAML support in the server.

**Sessions** use short-lived access tokens bound to the device's signing key with DPoP (RFC 9449, EdDSA as in RFC 8037). A token copied off a machine is useless without the device key.

**Machines** present a workload token, which the server maps to a machine principal through rules on issuer and subject:

- a Kubernetes projected service-account token with the audience `pasejo-server`, validated through the cluster issuer's OIDC discovery, or through TokenReview when the issuer is not reachable from the server;
- a GitHub Actions or GitLab CI OIDC token.

**Provisioning** uses SCIM 2.0 (RFC 7643, RFC 7644) for users and groups. Deactivating a user suspends their sessions at once and queues the cryptographic follow-up below.

## Revocation and rotation

Removing user *U* from store *S*, or deactivating *U*, runs in three steps:

1. **Access stops.** The server stops serving *S* to *U* immediately.
2. **New epoch.** A pending action asks a store admin, or the automation identity, to sign `store.revoke` and open epoch *e + 1*: new keys wrapped to the remaining devices. The next commit re-encrypts the index and the lookup object. Secret objects are re-encrypted when they are next written, or all at once with `pasejo store rekey`.
3. **Rotation list.** From the audit log, the server knows which identifiers *U* fetched and which existed while *U* had access. The admin's client shows their names. Rotation is the revocation that counts, because *U* may have kept plaintext.

Re-encrypting lazily gives up nothing against *U*: everything still encrypted under epoch *e* was readable by *U* before the removal. Organizations that also want old ciphertext unreadable to *U*'s keys, for example in case a database backup leaks, run `pasejo store rekey`.

**Time-bound grants.** A grant entry may carry an expiry. The server stops serving at expiry, and the epoch rotation of step 2 follows.

## Audit

**Events:**

- sign-ins and failed sign-ins;
- log entries;
- key, index and lookup fetches;
- object fetches, per identifier and version;
- commits, per changed or deleted identifier;
- audit queries and exports;
- administrative changes;
- every denied request.

**Fields:** time, actor, device, machine context, IP address, user agent, store, identifier, version, outcome and request id.

**Integrity:**

- Events live in an append-only table, written through a database role without `UPDATE` or `DELETE`.
- Each event carries the hash of its predecessor, and the server periodically signs a checkpoint with a key kept in a file, through PKCS#11, or in a cloud KMS. An organization can run a witness that co-signs checkpoints.
- Commits carry client signatures, so writes cannot be repudiated. Reads rest on the server's word.

**Names.** Events record identifiers. `pasejo audit log` resolves them with the auditor's names keys for every epoch. An auditor sees "alice read `prod/database` at 10:02" without being able to read the value.

**Limits** that compliance documentation has to state:

- reads served from a ciphertext cache;
- reads that the bridge makes for a workload, which are attributed to the machine and the tenant label it records;
- plaintext copied after decryption.

**Export** goes to OTLP logs, syslog (RFC 5424), JSON Lines in object storage, or a webhook. Retention is set per organization, and pruning keeps the checkpoint that covers the removed range.

**Browser host.** The host already records each reveal in its local release log (`browser/release_log.rs`). For a remote store, the reveal is also an object fetch the server audits, so the local log becomes a second record.

## Kubernetes

The bridge is a subcommand of the CLI, `pasejo bridge`, shipped as a container image. It runs in the cluster as a machine device:

- **Enrollment.** On first start, it generates its age identity and signing key, keeps them in a Kubernetes Secret, and requests enrollment with its service-account token. An approver signs `device.add`.
- **API.** It serves `GET /v1/secrets/{store}/{name}`, with a query parameter selecting a line or field, to external-secrets-operator's webhook provider.

The external-secrets.io documentation, as read on 2026-11-15 (the page footer says © 2025), states the following about the Generic Webhook provider:

- Its stability level is alpha.
- Templating covers `url`, `headers`, `body` and `result.jsonPath`, and `.remoteRef.key` is available. A templated host fails store validation, so only the path and query may be templated.
- Credentials come from Secrets labeled `external-secrets.io/type: webhook`, and server trust from `caBundle` or `caProvider`. Client certificates are not documented.
- Templates receive no metadata about the ExternalSecret that triggered the request.
- An HTTP 404 means the secret was deleted, which triggers the `deletionPolicy`.
- PushSecret is supported.

These facts shape the bridge:

- **Status codes.** The bridge answers 404 only for a name absent from an index it fetched and verified, and 503 whenever the server or the index is unreachable. A server outage must never delete Secrets in the cluster.
- **Attribution.** ESO does not say which ExternalSecret asked. One SecretStore per namespace, each with its own bridge credential, lets the bridge tag each read with the namespace, which the audit log records as machine context.
- **Grants.** The bridge is a machine principal with grants for only the stores its cluster needs.
- **PushSecret.** Writing generated credentials back from the cluster maps to commits by the bridge with a writer grant. It belongs to a later phase.

A native ESO provider would mean Go code inside ESO. A Secrets Store CSI driver provider would mount values as files without creating Kubernetes Secrets. Both can come later behind the same bridge API.

## Client changes

- **Remote registration.** `StoreRegistration` gains a remote variant: server, organization, store identifier and device. File stores are unchanged.
- **Backend trait.** Commands stop decrypting a whole `PasswordStore` and use a backend trait with name-level operations: list, get, put, rename, delete, advance an HOTP counter. The file backend implements the trait on `PasswordStore` inside the existing funnel, and the remote backend on the protocol. This is the largest client change: 22 calls of `with_store`, `with_store_then` and `open_store` in seven command modules, plus the browser host's own read path in `browser/host.rs`.
- **Existing interfaces.** `secret resolve`, `exec`, the fnox plugin and breto keep their interfaces. A resolve against a remote store fetches exactly the referenced objects.
- **Browser host.** A query reads the index and the lookup object; a reveal fetches one object.
- **Recipients and migration.** `pasejo recipient` does not apply to remote stores, whose membership is the log. `pasejo store migrate --to-remote` uploads a file store.
- **HTTP client.** The CLI talks HTTP through `ureq`, which it already depends on, so it needs no async runtime.

## Server architecture

- **Stack.** Rust with tokio and axum, and sqlx on PostgreSQL. SQLite serves evaluation and single-node installations.
- **Object storage.** Objects live in PostgreSQL, since secrets are small. External object storage can come later.
- **Scaling.** Server processes are stateless behind a load balancer, and high availability comes from PostgreSQL.
- **Operations.** Configuration comes from a file and environment variables. The server exposes Prometheus metrics and health and readiness endpoints, and emits OpenTelemetry traces.
- **Errors.** API errors are stable codes, and the CLI translates them into its eight languages, as it does for the browser host's `ErrorKind`.
- **Tables:** `orgs`, `users`, `user_identities`, `groups`, `group_members`, `devices`, `machines`, `machine_rules`, `stores`, `grants`, `log_entries`, `wrappings`, `index_versions`, `lookup_versions`, `objects`, `audit_events`, `audit_checkpoints`, `sessions`, `pending_actions`, `rotation_tasks`, `scim_tokens` and `access_reviews`. Each carries `org_id`.

## Repository layout

The server lives in this repository as a Cargo workspace, so the CLI and the server share the protocol code:

- **CLI.** The root package `pasejo` stays at the root, so `src/`, `tests/`, the trycmd paths under `docs/commands/` and the release workflow keep working.
- **`crates/pasejo-protocol`** holds wire types, object formats, envelope cryptography and log verification. It does no I/O and has no async code, and both the CLI and the server use it.
- **`crates/pasejo-server`** holds the server binary.

Consequences:

- **Workspace tests.** `cargo test` at the root builds the root package only, so CI passes `--workspace`.
- **Publishing.** Once the CLI depends on `pasejo-protocol`, crates.io needs that crate published first, through a multi-package `cargo publish --workspace`. The release's change detection then has to include `crates/pasejo-protocol`.
- **Shared lock file.** The server's dependencies (tokio, axum, sqlx) stay out of the CLI's build graph, but the workspace shares one `Cargo.lock`. A server dependency update changes `Cargo.lock`, which the CLI release counts as a change today, so its path filter needs refining.
- **Server releases.** The server gets its own release workflow: binaries and a container image, signed with cosign.

## Phases

1. **Protocol and client refactor.** The protocol crate: object formats, envelope cryptography and log verification, with test vectors. The client backend trait, with only the file backend, so behavior does not change.
2. **Server MVP.** OIDC sign-in, organizations, devices, the membership log, stores, commits, fetches and the audit log; the client's remote backend, `pasejo org` and `pasejo audit log`.
3. **Kubernetes.** Machine principals, workload identity, the bridge and its ESO documentation.
4. **Enterprise controls.** SCIM, the revocation workflow with rotation tasks, time-bound grants, access reviews, SIEM export and two-admin rules.
5. **Later work.** A web console, log witnesses, a CSI provider, PushSecret and hardware-backed signing keys.

## Open questions

- **Store labels.** Should store labels be encrypted too, trading console usability for less metadata?
- **Lookup keys.** Should they be a store setting, as proposed, or should clients keep `--uri-keys` and build per-client lookup objects?
- **Web console.** Where does it sign and decrypt names: with keys inside the page (WASM), or through the browser extension's native host, which already holds a channel to the local pasejo?
- **Store policies.** Password strength and required fields can only be enforced by clients, since the server sees no plaintext. Is client-side enforcement enough?
- **Break-glass.** An offline recipient, such as a printed age identity or a Shamir split, works without the server, which then cannot audit its use.
- **Distribution.** Which license, container registry and Helm chart repository should the server ship under?
