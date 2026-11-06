# `pasejo browser`

The `browser` subcommand groups commands that expose stored credentials to web browsers. Its help output looks like this:

```console
$ pasejo browser --help
Browser integration

Usage: pasejo browser [OPTIONS] <COMMAND>

Commands:
  query             Search a store for credentials matching a domain
  host              Run a native-messaging host that browser extensions connect to
  install-manifest  Install the native-messaging manifest for one or more browsers
  log               Show which credentials the native-messaging host released to which site
  help              Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`browser query` searches a single store for credentials matching the given domain and prints the results as JSON on stdout. Each result names the secret, its store, its username and the matching URL, together with the kind and score of the match and whether the secret asks for confirmation before a fill (`confirm`, see below); it carries no password, no other field values and no one-time code. `pasejo secret show` and `pasejo otp show` print those. URL field matches (against any field listed in `--uri-keys`) are scored higher than fallback matches against the secret's path segments. Leading and trailing whitespace is removed from the values of fields listed in `--uri-keys` and `--username-keys`, so an aligned `login:    alice` yields the username `alice`.

`browser host` runs the same matching logic as a long-lived native-messaging host: the browser spawns this process per its native-messaging manifest, exchanges length-prefixed JSON messages over stdin/stdout, and closes the pipe when the user closes the browser. The host refuses to start when stdin is a terminal — it is not meant to be run by hand. Store selection follows the same rules as every other pasejo command (`PASEJO_DEFAULT_STORE` env, the config's `default_store` field, then the first store); the manifest pins a specific configuration and store through the `--env` flag of `install-manifest` (below), which bakes them into the generated wrapper.

The host releases credentials in two steps. A `query` returns the same metadata as `browser query`, so the browser extension can list the candidates without receiving any secret. Only when you pick one does the extension send a `reveal` for that store, secret name and the domain of the page; the host decrypts the store again, re-runs the matcher for that domain, and returns the username and password only if the secret is among the matches. A secret that does not match the domain, or that does not exist, is answered with `not_matched`. The current TOTP code is included in a reveal only when one of the secret's URL fields names the page's host exactly; a match through a sibling subdomain of the same registrable domain, or through the secret's name, releases the password without the code. HOTP codes are never released to the browser, because emitting one without advancing its counter would break its single-use guarantee.

A secret can restrict how it reaches a browser through two reserved fields in its body, written like any other `key: value` line below the password:

```text
hunter2
url: https://login.example.com
pasejo-fill: exact-host
pasejo-confirm: always
```

`pasejo-fill: never` keeps the secret out of `browser query` and the host's `query` and `reveal` entirely. `pasejo-fill: exact-host` admits the secret only where one of its URL fields names the page's host exactly, so neither a sibling subdomain of the same registrable domain nor a match through the secret's name finds it. Any other `pasejo-fill` value is treated as `never`, so a misspelt restriction refuses the browser instead of widening what it receives; when the field repeats, the strictest value applies. Both values are enforced by pasejo itself, on every query and again on every reveal. `pasejo-confirm: always` sets `"confirm": true` on the secret's query result, and the browser extension asks you before it fills that credential; any other value of the field is read as `always`. That confirmation is enforced by the extension, not by pasejo: the host only reports the flag and still answers a `reveal` for the secret. Field names compare case-insensitively. Both fields stay ordinary fields everywhere else: `secret show` prints them and `export bitwarden` exports them unchanged as custom fields.

The host decrypts the store on every query and every reveal and cannot ask for a passphrase, since the browser gives it no terminal. It therefore skips passphrase-protected identities — age identity files encrypted with a passphrase (`age --passphrase`) and encrypted SSH private keys — and answers `identity_locked` when the remaining identities cannot decrypt the store. An ssh-agent does not help here, because pasejo reads SSH keys from their files and never asks an agent. For browser use, register an unencrypted identity file (protected by restrictive file permissions and disk encryption) or a plugin-backed identity such as one from `age-plugin-yubikey`. A plugin that asks for a PIN does so through pinentry when one is available and falls back to the terminal otherwise; for the browser-spawned host, set `PINENTRY_PROGRAM` to a graphical pinentry, e.g. `--env PINENTRY_PROGRAM=/usr/bin/pinentry-gnome3`.

Every credential the host releases is recorded first in a local release log, and `browser log` prints it. Each entry names the time (UTC), the domain the credential was released for, the store, the secret's name and the calling extension: the `chrome-extension://…` origin for chromium-family browsers, or the add-on id for Firefox, and `-` when the browser's arguments did not reach the host. Passwords, one-time codes and other field values are never written to the log. When an entry cannot be written, the host withholds the credential and answers `release_log_failed`, so no release goes unrecorded. Queries and refused reveals are not logged.

```console
$ pasejo browser log
Time	Domain	Store	Secret	Caller
2025-11-06T12:00:00Z	example.com	personal	alice	chrome-extension://abcdefgh/

```

The columns are separated by tabs, so the output can be piped into `column -t` or `cut`. The log lives at `pasejo/browser-releases.jsonl` in the local data directory (`~/.local/share` on Linux, or the directory named by `PASEJO_DATA_DIR`), one JSON object per line, readable by its owner only on Unix. It is never synchronised by hooks. Once it would grow beyond 1 MiB it is renamed to `browser-releases.jsonl.1`, replacing the previous rotation, and `browser log` reads both files.

`browser host` checks which extension the browser started it for when it is given one or more `--allowed-caller` values. Browsers name the caller in the arguments they pass to the host: chromium-family browsers pass the extension's origin, `chrome-extension://<id>/` with a trailing slash, as the first argument on every OS (followed on Windows by `--parent-window=<handle>`), and Firefox passes the path of the native-messaging manifest followed by the add-on id on every OS. The host compares that identity exactly against its `--allowed-caller` values, so they take the same form as the manifest's `allowed_origins` or `allowed_extensions` entries. An extension that is not listed, or a browser that names no extension at all, gets `caller_not_allowed` with a hint naming the extension in reply to every request, including `list_stores`; the host reads no store for it and writes the same hint to stderr. Without `--allowed-caller` the host accepts every caller, and only the browser's own check of the manifest's allowlist applies.

`browser install-manifest` writes the per-user native-messaging manifest a browser needs before it will spawn `pasejo browser host`. Pass `--browser firefox` and/or one or more chromium-family entries (`chrome`, `chromium`, `brave`, `edge`, `vivaldi`, `opera`); for chromium-family browsers, also pass `--extension-id` because their extension ids are derived from the unpacked extension's public key. The Firefox build of the [pasejo web extension][ext] ships with a stable add-on id, so `--extension-id` is optional there. The command also drops a tiny wrapper script per browser under the user's data directory (`native-messaging/<browser>/wtf.metio.pasejo`, with a `.cmd` suffix on Windows) that invokes `pasejo browser host` on behalf of the browser. The wrapper passes one `--allowed-caller` for each extension the browser's manifest admits — the `chrome-extension://<id>/` origins in `allowed_origins`, or the add-on ids in `allowed_extensions` — followed by the arguments the browser adds, which name the calling extension, so the host refuses any other extension even when another manifest points at the same wrapper. Because each browser has its own wrapper, running `install-manifest` once for Firefox and once for a chromium-family browser leaves both working. Extension ids may contain only ASCII letters, digits, `@`, `.`, `_`, `-` and curly braces, the characters chromium ids and Firefox add-on ids consist of. Use `--print` to emit the JSON to stdout (no files written) for inspection or templating.

`--binary` points the manifest at your own launcher instead and writes no wrapper. Such a launcher has to forward the browser's arguments to `pasejo browser host`, or the host does not learn its caller, and it should pass `--allowed-caller` itself before them, in the same form the manifest lists the extension; without `--allowed-caller` the host accepts every caller. The flags must come before the browser's arguments, because the host reads everything from the first browser argument on as the browser's. For a chromium-family browser on Linux or macOS:

```sh
#!/bin/sh
exec /usr/local/bin/pasejo browser host --allowed-caller 'chrome-extension://abcdefghijklmnopabcdefghijklmnop/' "$@"
```

and for Firefox on Windows, as a `.cmd` file:

```bat
@echo off
"C:\Program Files\pasejo\pasejo.exe" browser host --allowed-caller "pasejo@metio.wtf" %*
```

This works on Linux, macOS, and Windows. On Linux and macOS the manifest is written into the browser's per-user lookup directory; on Windows — where browsers find native-messaging hosts through the registry — the manifest is written next to the wrapper under the data directory (`native-messaging/<browser>/wtf.metio.pasejo.json`) and an `HKEY_CURRENT_USER` value is registered to point at it, and the wrapper is a `.cmd` batch file rather than a shell script.

A browser-spawned host inherits a stripped environment — `$HOME` is set, but `$PASEJO_CONFIG`, `$PASEJO_DEFAULT_STORE`, and `$PINENTRY_PROGRAM` usually are not, so the host cannot find a non-default configuration or store, or a graphical pinentry. Rather than hand-writing a launcher, pass `--env KEY=VALUE` (repeatable) to bake those variables into the generated wrapper — e.g. `--env PASEJO_DEFAULT_STORE=work --env PINENTRY_PROGRAM=/usr/bin/pinentry-gnome3`. `--env` has no effect together with `--print` or `--binary`, because neither generates a wrapper. `--host-name` overrides the native-messaging host name (default `wtf.metio.pasejo`); change it only when integrating with a non-default extension build. When more than one `--browser` is given, `--extension-id` is rejected, since a chromium id written into Firefox's gecko-id slot would silently never authorize the real add-on.

[ext]: https://github.com/metio/pasejo-web-extension
