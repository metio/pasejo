# Using pasejo with fnox

[fnox](https://fnox.jdx.dev/) sets environment variables for a command from the secrets a `fnox.toml` declares, and its Lua plugin provider can read them from any source. The plugin in [`contrib/fnox/provider.lua`](../../contrib/fnox/provider.lua) reads them from a pasejo store through [`pasejo secret resolve`](../commands/pasejo-cmd-secret.md), so it needs a pasejo release that has that command.

## Installation

fnox installs plugins from a Git repository only when `provider.lua` sits at the repository's root, which is not the case for the pasejo repository. Link a checkout instead:

```sh
fnox plugin link pasejo /path/to/pasejo/contrib/fnox
```

Alternatively, point the provider's `plugin` field at the file itself, as in `plugin = "/path/to/pasejo/contrib/fnox/provider.lua"`.

## Configuration

```toml
[providers.pasejo]
type = "lua"
plugin = "pasejo"
options = { store = "work" }

[secrets]
GITHUB_TOKEN = { provider = "pasejo", value = "api/github.com/token", if_missing = "error" }
DB_USER = { provider = "pasejo", value = "db/prod#username", if_missing = "error" }
```

```sh
fnox exec -- ./deploy.sh
```

A `value` is the path of a secret, which selects its first line, the password by pasejo's convention. A path followed by `#FIELD` selects the value of that `FIELD: value` line instead; the path runs up to the last `#`. The `options` are all optional:

- `store`: the pasejo store to read. Without it, pasejo's default store applies. Declare one provider per store to read from several.
- `bin`: the pasejo executable. Without it, fnox runs `pasejo` from `PATH`.
- `offline`: `"true"` skips the store's pull hooks.

pasejo runs with fnox's environment, so `PASEJO_CONFIG` and `PASEJO_DEFAULT_STORE` apply as usual.

## Behavior

fnox hands the plugin every reference of a provider at once, and the plugin passes them to a single `pasejo secret resolve`. The store is therefore decrypted once per `fnox exec`, which matters for hardware keys that ask for a touch per decryption.

pasejo resolves all references or none. When one secret or field is missing, every secret of that provider fails, and the error names the reference as written in `fnox.toml`. fnox's own default for a failed secret is a warning, after which it runs the command without the variable. Set `if_missing = "error"` on each secret, or pass `--if-missing error`, so the command does not start with values missing.
