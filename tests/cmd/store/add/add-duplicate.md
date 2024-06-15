```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name same
info: Store 'same' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store add --path different --name same
? 1
Error: Store name already exists. Please use a different name.

```
