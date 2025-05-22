```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store exec -- sdglsdfgksjdfgl lgkhsdjfghsdfg
? 1
Error: Failed to run command sdglsdfgksjdfgl

Caused by:
    No such file or directory (os error 2)

```
