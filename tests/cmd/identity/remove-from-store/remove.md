```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --alias default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --alias other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --file some/file/location
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity remove --store default --file some/file/location
Identity removed

```
