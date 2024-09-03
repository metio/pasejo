```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --alias default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo identity add --file some/file/location
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --file /an/absolute/path
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --file /annother/path
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity remove --file /an/absolute/path
Identity removed

$ PASEJO_CONFIG=config.toml pasejo identity remove --file some/file/location
Identity removed

```
