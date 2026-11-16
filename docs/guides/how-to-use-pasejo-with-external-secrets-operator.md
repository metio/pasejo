# Using pasejo with external-secrets-operator

`pasejo-bridge` runs in a Kubernetes cluster as a machine device of one organization on a pasejo server and serves single secret values to the [Generic Webhook provider](https://external-secrets.io/latest/provider/webhook/) of external-secrets-operator (ESO). The bridge decrypts on its own, so the server never sees a value, and the server records every read under the tenant whose credential the request carried.

## Image

Each release publishes the bridge as a signed container image, `ghcr.io/metio/pasejo-bridge`, and as statically linked Linux binaries for x86_64 and aarch64 on the [releases page](https://github.com/metio/pasejo/releases). The image runs as uid 65532 without a shell and starts `pasejo-bridge serve`, which reads `/etc/pasejo-bridge/config.toml` unless `--config` or `PASEJO_BRIDGE_CONFIG` names another file. Verify it with cosign:

```sh
cosign verify ghcr.io/metio/pasejo-bridge:<version> \
  --certificate-identity-regexp '^https://github.com/metio/pasejo/.github/workflows/release.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

## Device keys and credentials

Create the bridge's device keys once and store them in a Secret:

```sh
pasejo-bridge keygen --output device.json
kubectl create secret generic pasejo-bridge-device --from-file=device.json
```

`keygen` prints the device's fingerprint, which the approver compares later.

Create one credential per tenant, typically one per namespace:

```sh
pasejo-bridge credential --tenant team-a
```

The command prints a bearer token and a `[[credentials]]` block. The token goes into the tenant's namespace; the block goes into the bridge's configuration, which keeps only the token's SHA-256.

## Configuration

```toml
server = "https://pasejo.example"
org = "<organization identifier>"
genesis = "<genesis hash>"
device_file = "/etc/pasejo-bridge/device/device.json"
workload_token_file = "/var/run/secrets/pasejo/token"
label = "cluster-1"

[[credentials]]
tenant = "team-a"
token_sha256 = "..."
```

`org` and `genesis` are the `id` and `genesis` values that `pasejo org join` stored for the organization in the `[[servers.orgs]]` table of your pasejo configuration. The bridge checks the membership log against that genesis hash on every read.

The bridge signs in with a projected service-account token whose audience is `pasejo-server`. The server's `[[workload]]` configuration maps the service account to a machine name, for example `system:serviceaccount:pasejo:bridge` to `cluster-1`. Mount the token, the device Secret and the configuration into the bridge's pod and start it with `pasejo-bridge serve --config /etc/pasejo-bridge/config.toml`. It listens on port 8080 unless `listen` says otherwise.

## Approval and access

On its first start the bridge enrolls itself and answers every read with 503 until it is approved. An admin then admits it like any device and grants it the stores it serves:

```sh
pasejo org role machine:cluster-1 member
pasejo org approve <device> --fingerprint "<fingerprint from keygen>"
pasejo store grant machine:cluster-1 reader --store prod
```

`/readyz` answers 200 once the device is active and 503 before; `/healthz` answers 200 while the process runs.

## SecretStore

Each namespace gets its own SecretStore and credential, which gives every read the namespace's tenant label:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: pasejo-bridge-credential
  namespace: team-a
  labels:
    external-secrets.io/type: webhook
stringData:
  token: "<token printed by pasejo-bridge credential>"
---
apiVersion: external-secrets.io/v1
kind: SecretStore
metadata:
  name: pasejo
  namespace: team-a
spec:
  provider:
    webhook:
      url: "http://pasejo-bridge.pasejo.svc:8080/v1/secrets/prod/{{ .remoteRef.key }}"
      result:
        jsonPath: "$.value"
      headers:
        Authorization: "Bearer {{ .auth.token }}"
      secrets:
        - name: auth
          secretRef:
            name: pasejo-bridge-credential
---
apiVersion: external-secrets.io/v1
kind: ExternalSecret
metadata:
  name: database
  namespace: team-a
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: pasejo
    kind: SecretStore
  target:
    name: database
  data:
    - secretKey: password
      remoteRef:
        key: db/production
```

The path names the store by its label or identifier, followed by the secret's name, which may contain `/`. The reply is `{"value": "..."}` with the whole secret. The query parameter `line=N` selects one line (negative numbers count from the end) and `field=KEY` the value of a `KEY: value` field, as `pasejo secret show --line` and `--field` do. ESO passes no property to the template, so a field has to travel in the key itself, for example `key: "db/production?field=username"`; this relies on ESO inserting the key into the URL unescaped, which this guide has not verified for every ESO version.

## Status codes

| Status | Meaning | ESO's reaction |
|---|---|---|
| 200 | the value | writes it to the target Secret |
| 400 | malformed query parameters | error, retried |
| 401 | missing or unknown credential | error, retried |
| 404 | the name is absent from the store's verified index | runs the ExternalSecret's `deletionPolicy` |
| 422 | the secret has no such line or field | error, retried |
| 503 | the server, the approval, the store, its keys or its index are unavailable | error, retried |

The bridge answers 404 only for a name it looked up in an index it fetched and verified. An unknown store, a revoked grant or a server outage answers 503, so none of them deletes Secrets in the cluster.

## Limits

The bridge holds a plaintext value only while it answers one request. Once ESO has written the value to a Kubernetes Secret, Kubernetes' own controls protect it. The audit trail names the tenant label of the credential, not the ExternalSecret, because ESO does not tell the webhook which ExternalSecret asked.
