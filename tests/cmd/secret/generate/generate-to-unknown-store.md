```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo secret generate some-secret --store another
? 2
error: invalid value 'another' for '--store <STORE>': Store with name 'another' does not exist in configuration

For more information, try '--help'.

```
