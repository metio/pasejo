```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --name other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --file some/file/location
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --store non-existing --file some/file/location
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --global --file some/other/location
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo identity add --file <FILE> --store <STORE>

For more information, try '--help'.

```
