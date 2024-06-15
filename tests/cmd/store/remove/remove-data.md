```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store remove something --remove-data 
info: Store 'something' removed

$ PASEJO_CONFIG=config.toml pasejo store add --path store --name another
info: Store 'another' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store remove --remove-data another
info: Store 'another' removed

```
