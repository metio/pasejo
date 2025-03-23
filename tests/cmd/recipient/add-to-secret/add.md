```console
$ PASEJO_CONFIG=config.toml pasejo store init --path store --name default --vcs none
info: Store initialized at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo recipient add --path some/secret/name --public-key "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"
? 2
error: invalid value 'some/secret/name' for '--path <PATH>': path does not match any secret or folder in the store

Usage: pasejo recipient add [OPTIONS] <--public-key <PUBLIC_KEY>|--file <FILE>|--codeberg <CODEBERG>|--github <GITHUB>|--gitlab <GITLAB>>

For more information, try '--help'.

```
