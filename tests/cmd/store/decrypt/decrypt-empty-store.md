```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store decrypt
? 1
Error: Cannot decrypt store

Caused by:
    0: Cannot read file at "[CWD]/store"
    1: No such file or directory (os error 2)

```
