```console
$ PASEJO_CONFIG=config.toml pasejo store init --path store --name default
info: Store initialized at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo recipient add --path some/secret/name --public-key "age12abcdefghijklmnopqrstuvwxyz"
? 2
error: invalid value 'some/secret/name' for '--path <PATH>': path does not match any secret or folder in the store

Usage: pasejo recipient add [OPTIONS] <--public-key <PUBLIC_KEY>>

For more information, try '--help'.

```
