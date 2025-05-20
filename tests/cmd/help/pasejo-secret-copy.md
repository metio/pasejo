```
$ pasejo secret help copy
Copy secret from old-path to new-path

Usage: pasejo secret copy [OPTIONS] <SOURCE_PATH> <TARGET_PATH>

Arguments:
  <SOURCE_PATH>  The path of an existing secret
  <TARGET_PATH>  The target path for the copied secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret copy --help
Copy secret from old-path to new-path

Usage: pasejo secret copy [OPTIONS] <SOURCE_PATH> <TARGET_PATH>

Arguments:
  <SOURCE_PATH>  The path of an existing secret
  <TARGET_PATH>  The target path for the copied secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret copy -h
Copy secret from old-path to new-path

Usage: pasejo secret copy [OPTIONS] <SOURCE_PATH> <TARGET_PATH>

Arguments:
  <SOURCE_PATH>  The path of an existing secret
  <TARGET_PATH>  The target path for the copied secret

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force          Overwrite an existing secrets without prompting
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```
