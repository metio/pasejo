# `pasejo browser`

The `browser` subcommand groups commands that expose stored credentials to web browsers. Its help output looks like this:

```console
$ pasejo browser --help
Browser integration

Usage: pasejo browser [OPTIONS] <COMMAND>

Commands:
  query  Search a store for credentials matching a domain
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`browser query` searches a single store for credentials matching the given domain and prints the results as JSON on stdout. URL field matches (against any field listed in `--uri-keys`) are scored higher than fallback matches against the secret's path segments.
