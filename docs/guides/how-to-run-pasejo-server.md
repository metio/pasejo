# Running pasejo-server

`pasejo-server` holds remote stores for teams: it keeps encrypted objects, an encrypted index and the signed membership log of each organization, signs people in through your identity provider and records every access in a tamper-evident audit trail. It never sees a secret, a secret name or a store name. The server is licensed AGPL-3.0-or-later.

Each release publishes the server as a signed container image, `ghcr.io/metio/pasejo-server`, and as statically linked Linux binaries for x86_64 and aarch64 on the [releases page](https://github.com/metio/pasejo/releases). The image runs as uid 65532 without a shell, reads its configuration from `/etc/pasejo-server/config.toml` and starts `pasejo-server serve`.

## Verifying the image

Images are signed with cosign keyless by the release workflow:

```sh
cosign verify ghcr.io/metio/pasejo-server:<version> \
  --certificate-identity-regexp '^https://github.com/metio/pasejo/.github/workflows/release.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

The release binaries are covered by the signed `SHA256SUMS` file of the release.

## Master key

The master key derives the server's checkpoint signing key and the key that encrypts the break-glass account's TOTP secret. Create it once and keep a copy offline: a server with a different master key signs with a different checkpoint key, which every client refuses, and cannot verify the break-glass account's codes.

```sh
podman run --rm ghcr.io/metio/pasejo-server:<version> keygen | podman secret create pasejo-master-key -
```

## Configuration

A minimal configuration with SQLite:

```toml
public_url = "https://pasejo.example.com"
database_url = "sqlite:///var/lib/pasejo-server/pasejo.db?mode=rwc"
master_key_file = "/etc/pasejo-server/master.key"

[oidc]
issuer = "https://id.example.com"
client_id = "pasejo"
client_secret_file = "/etc/pasejo-server/oidc-secret"
```

PostgreSQL works the same way with `database_url = "postgres://user:password@host/pasejo"`. The server applies its migrations on start; `pasejo-server migrate` applies them without serving.

Register `<public_url>/auth/callback` as the redirect URI at your identity provider. The server listens on `0.0.0.0:8080` unless `listen` says otherwise, and terminates TLS itself when `[tls]` names a `certificate` and a `key`; behind a reverse proxy, list the proxy's networks in `trusted_proxies` so the server takes the client's address from the forwarded headers for audit events, sign-in rate limits and the break-glass networks.

The other tables are optional:

| Table | Purpose |
|---|---|
| `[[workload]]` | Workload token issuers, such as a Kubernetes cluster, with rules mapping subjects to machine users |
| `[break_glass]` | `enabled` and the `networks` the break-glass account may sign in from (loopback by default) |
| `[audit]` | Checkpoint cadence, `retention_days`, a copy of every event on stdout, and a signed webhook |
| `[orgs]` | Who may create organizations: `anyone`, `nobody` or the `listed` `creators` |
| `[limits]` | Sign-in attempts per minute and request body sizes |
| `metrics_listen` | A separate address serving Prometheus metrics; the main listener never serves them |

## Running the container

The server runs as uid 65532, so the key files it reads must be readable by that uid. Podman secrets mounted with that owner keep them unreadable to everyone else:

```sh
podman secret create pasejo-oidc-secret oidc-secret
podman volume create pasejo-server
podman run --detach --name pasejo-server \
  --publish 8080:8080 \
  --volume pasejo-server:/var/lib/pasejo-server \
  --volume ./config.toml:/etc/pasejo-server/config.toml:ro,Z \
  --secret pasejo-master-key,target=/etc/pasejo-server/master.key,uid=65532,mode=0400 \
  --secret pasejo-oidc-secret,target=/etc/pasejo-server/oidc-secret,uid=65532,mode=0400 \
  ghcr.io/metio/pasejo-server:<version>
```

`/healthz` answers 200 while the process runs and `/readyz` answers 200 while the database is reachable, which suits liveness and readiness probes.

The administrative commands run in the same image against the same configuration:

```sh
podman exec pasejo-server pasejo-server break-glass create
podman exec pasejo-server pasejo-server audit prune --days 400
```

`break-glass create` prints the account's password and TOTP secret once. The account is independent of your identity provider and signs in on `<public_url>/console/break-glass.html` from the networks `[break_glass]` allows.

## Next steps

Register a device with `pasejo server login <public_url>` and create an organization with `pasejo org create`; see [pasejo server](../commands/pasejo-cmd-server.md) and [pasejo org](../commands/pasejo-cmd-org.md). The browser console is served at `<public_url>/console/`. To serve secrets to Kubernetes, see [Use pasejo with external-secrets-operator](./how-to-use-pasejo-with-external-secrets-operator.md).
