```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store exec -- printenv PASEJO_EXEC_STORE_PATH
[CWD]/store

$ PASEJO_CONFIG=config.toml pasejo store exec -- printenv PASEJO_EXEC_STORE_PARENT
[CWD]

$ PASEJO_CONFIG=config.toml pasejo store exec -- printenv PASEJO_EXEC_COMMAND
printenv

```
