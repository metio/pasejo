# pasejo commands

The `pasejo` command-line tool provides a set of commands to manage secrets, identities, recipients, and configuration options. This document provides an overview of the available commands and their usage.

The following commands are available in `pasejo`:

- [browser](./pasejo-cmd-browser.md): Browser integration (query, native-messaging host, manifest install)
- [config](./pasejo-cmd-config.md): Manage `pasejo` configuration
- [export](./pasejo-cmd-export.md): Export passwords to another password manager
- [hook](./pasejo-cmd-hook.md): Manage synchronization hooks
- [identity](./pasejo-cmd-identity.md): Manage identities
- [otp](./pasejo-cmd-otp.md): Manage one-time passwords
- [recipient](./pasejo-cmd-recipient.md): Manage recipients
- [secret](./pasejo-cmd-secret.md): Manage secrets
- [store](./pasejo-cmd-store.md): Manage stores

## Environment variables

`pasejo` reads a few environment variables in addition to its command-line flags:

- `PASEJO_CONFIG`: path to the configuration file. Overrides the OS-standard config directory.
- `PASEJO_DEFAULT_STORE`: name of the store to use when no `--store` is given. Takes precedence over the config's `default_store`.
- `PASEJO_DATA_DIR`: directory for pasejo's local data (e.g. hook throttle timestamps). Overrides the OS-standard data directory.
- `PASEJO_DISABLE_HOOK_THROTTLING`: when set, pull/push hooks run on every invocation instead of being rate-limited by their configured interval.
- `CODEBERG_HOST`, `GITHUB_HOST`, `GITLAB_HOST`: override the host the `recipient add --codeberg/--github/--gitlab` username lookups query, for self-hosted instances. A bare host (`git.example.com`) keeps the default `https://` scheme; include a scheme (`http://git.example.com:3000`) to reach an instance over plain HTTP or a non-standard port.

## Concurrency

Operations on a **store file** (add/show/edit a secret, OTP, recipient, merge, …) take an exclusive advisory file lock for the decrypt → mutate → encrypt window, so concurrent pasejo invocations against the same store are safe and never lose a write.

The **configuration file** (the registry of stores, identities, and hooks that `store add` / `store remove` / `store set-default` and `identity` / `recipient` / `hook` edits write) is **not** locked. Running two configuration-mutating commands at the very same moment is unsupported: each reads the config, applies its change, and rewrites the whole file, so the later writer wins and the earlier change is lost. The writes are individually atomic (no torn or truncated file), and a single human running commands one after another is never affected — only genuinely simultaneous mutations race. Serialize configuration changes if you script them.
