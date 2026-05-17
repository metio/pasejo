# `pasejo browser`

The `browser` subcommand groups commands that expose stored credentials to web browsers. Its help output looks like this:

```console
$ pasejo browser --help
Browser integration

Usage: pasejo browser [OPTIONS] <COMMAND>

Commands:
  query  Search a store for credentials matching a domain
  host   Run a native-messaging host that browser extensions connect to
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`browser query` searches a single store for credentials matching the given domain and prints the results as JSON on stdout. URL field matches (against any field listed in `--uri-keys`) are scored higher than fallback matches against the secret's path segments.

`browser host` runs the same matching logic as a long-lived native-messaging host: the browser spawns this process per its native-messaging manifest, exchanges length-prefixed JSON messages over stdin/stdout, and closes the pipe when the user closes the browser. The host refuses to start when stdin is a terminal — it is not meant to be run by hand. Store selection follows the same rules as every other pasejo command (`PASEJO_DEFAULT_STORE` env, the config's `default_store` field, then the first store); native-messaging manifests can pin a specific store by setting `PASEJO_DEFAULT_STORE` in the spawn environment.
