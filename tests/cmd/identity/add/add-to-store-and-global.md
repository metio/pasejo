```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something --global
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo identity add --file <FILE> --store <STORE>

For more information, try '--help'.

```
