```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store add --path path/to/another --name another --default
info: Store 'another' added at '[CWD]/path/to/another'
info: Store 'another' is now the default

$ PASEJO_CONFIG=config.toml pasejo store add --path somewhere/else --name other
info: Store 'other' added at '[CWD]/somewhere/else'

$ PASEJO_CONFIG=config.toml pasejo store list
something: [CWD]/store
another: [CWD]/path/to/another (default)
other: [CWD]/somewhere/else

```
