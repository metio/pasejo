```
$ pasejo secret help move
Move secret from old-path to new-path

Usage: pasejo secret move [OPTIONS] <CURRENT_PATH> <NEW_PATH>

Arguments:
  <CURRENT_PATH>  The current path of the secret
  <NEW_PATH>      The new path of the secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret move --help
Move secret from old-path to new-path

Usage: pasejo secret move [OPTIONS] <CURRENT_PATH> <NEW_PATH>

Arguments:
  <CURRENT_PATH>  The current path of the secret
  <NEW_PATH>      The new path of the secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret move -h
Move secret from old-path to new-path

Usage: pasejo secret move [OPTIONS] <CURRENT_PATH> <NEW_PATH>

Arguments:
  <CURRENT_PATH>  The current path of the secret
  <NEW_PATH>      The new path of the secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```
