```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store decrypt --store unknown-store
? 2
error: invalid value 'unknown-store' for '--store <STORE>': Store with name 'unknown-store' does not exist in configuration

For more information, try '--help'.

```
