# `pasejo secret`

The `secret` subcommand is used to manage secrets. Its help output looks like this:

```console
$ pasejo secret --help
Manage secrets

Usage: pasejo secret [OPTIONS] <COMMAND>

Commands:
  add       Add a new secret or overwrite an existing one
  audit     Audit password strength of secrets
  copy      Copy secret from old-path to new-path
  edit      Edit an existing secret
  generate  Generate a secret and add it into the store
  grep      Grep for a search-string in secrets when decrypted
  list      List all secrets
  move      Move secret from old-path to new-path
  remove    Remove an existing secret
  show      Show secret
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```


