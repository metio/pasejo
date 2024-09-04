```console
$ pasejo secret help insert
Insert a new secret or overwrite an existing one

Usage: pasejo secret insert [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -m, --multiline      Toggle multiline edit mode
  -f, --force          Toggle prompt for overwrites of existing secrets
  -i, --inherit        Ignore existing recipients of existing secrets and inherit recipients from nearest parent folder
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help           Print help

```

```console
$ pasejo secret insert --help
Insert a new secret or overwrite an existing one

Usage: pasejo secret insert [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -m, --multiline      Toggle multiline edit mode
  -f, --force          Toggle prompt for overwrites of existing secrets
  -i, --inherit        Ignore existing recipients of existing secrets and inherit recipients from nearest parent folder
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help           Print help

```

```console
$ pasejo secret insert -h
Insert a new secret or overwrite an existing one

Usage: pasejo secret insert [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -m, --multiline      Toggle multiline edit mode
  -f, --force          Toggle prompt for overwrites of existing secrets
  -i, --inherit        Ignore existing recipients of existing secrets and inherit recipients from nearest parent folder
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help           Print help

```
