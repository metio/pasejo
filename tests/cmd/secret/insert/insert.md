```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name something
info: Store initialized at '[CWD]/path/to/store'

$ PASEJO_CONFIG=config.toml pasejo secret insert --recipient abc --inherit path/to/secret
? 2
error: the argument '--recipient <RECIPIENT>' cannot be used with '--inherit'

Usage: pasejo secret insert --recipient <RECIPIENT> <SECRET_PATH>

For more information, try '--help'.

$ PASEJO_CONFIG=config.toml pasejo secret insert --inherit --recipient abc path/to/secret
? 2
error: the argument '--inherit' cannot be used with '--recipient <RECIPIENT>'

Usage: pasejo secret insert --inherit <SECRET_PATH>

For more information, try '--help'.

```
