```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default --default
Store initialized at [CWD]/path/to/store
Store default is now the default

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --name other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --file some/file/location
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --file some/default/location
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity remove --store default --file some/file/location
Identity removed

$ PASEJO_CONFIG=config.toml pasejo identity remove --file some/default/location
Identity removed

$ PASEJO_CONFIG=config.toml pasejo identity remove --store non-existing --file some/file/location
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```
