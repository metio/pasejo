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
  help              Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`browser query` searches a single store for credentials matching the given domain and prints the results as JSON on stdout. URL field matches (against any field listed in `--uri-keys`) are scored higher than fallback matches against the secret's path segments.

`browser host` runs the same matching logic as a long-lived native-messaging host: the browser spawns this process per its native-messaging manifest, exchanges length-prefixed JSON messages over stdin/stdout, and closes the pipe when the user closes the browser. The host refuses to start when stdin is a terminal — it is not meant to be run by hand. Store selection follows the same rules as every other pasejo command (`PASEJO_DEFAULT_STORE` env, the config's `default_store` field, then the first store); the manifest pins a specific store, identity, and ssh-agent socket through the `--env` flag of `install-manifest` (below), which bakes them into the generated wrapper.

`browser install-manifest` writes the per-user native-messaging manifest a browser needs before it will spawn `pasejo browser host`. Pass `--browser firefox` and/or one or more chromium-family entries (`chrome`, `chromium`, `brave`, `edge`, `vivaldi`, `opera`); for chromium-family browsers, also pass `--extension-id` because their extension ids are derived from the unpacked extension's public key. The Firefox build of the [pasejo web extension][ext] ships with a stable add-on id, so `--extension-id` is optional there. The command also drops a tiny wrapper script under the user's data directory that invokes `pasejo browser host` on behalf of the browser; pass `--binary` to point the manifest at your own launcher instead. Use `--print` to emit the JSON to stdout (no files written) for inspection or templating.

This works on Linux, macOS, and Windows. On Linux and macOS the manifest is written into the browser's per-user lookup directory; on Windows — where browsers find native-messaging hosts through the registry — the manifest is written under the data directory and an `HKEY_CURRENT_USER` value is registered to point at it, and the wrapper is a `.cmd` batch file rather than a shell script.

A browser-spawned host inherits a stripped environment — `$HOME` is set, but `$PASEJO_CONFIG`, `$PASEJO_DEFAULT_STORE`, and `$SSH_AUTH_SOCK` usually are not, so the host cannot reach an ssh-agent identity or a non-default store. Rather than hand-writing a launcher, pass `--env KEY=VALUE` (repeatable) to bake those variables into the generated wrapper — e.g. `--env SSH_AUTH_SOCK=$SSH_AUTH_SOCK --env PASEJO_DEFAULT_STORE=work`. `--env` has no effect together with `--print` or `--binary`, because neither generates a wrapper. `--host-name` overrides the native-messaging host name (default `wtf.metio.pasejo`); change it only when integrating with a non-default extension build. When more than one `--browser` is given, `--extension-id` is rejected, since a chromium id written into Firefox's gecko-id slot would silently never authorize the real add-on.

[ext]: https://github.com/metio/pasejo-web-extension
