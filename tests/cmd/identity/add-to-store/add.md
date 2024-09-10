```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default --default
Store initialized at [CWD]/path/to/store
Store default is now the default

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --name other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --file some-identity
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --store other --file some-identity
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity add --file other-identity
Identity added

```

```
$ PASEJO_CONFIG=config.toml pasejo identity add --file other-identity
? 2
error: invalid value 'other-identity' for '--file <FILE>': file was already added as an identity

Usage: pasejo identity add [OPTIONS] --file <FILE>

For more information, try '--help'.

```

```
$ PASEJO_CONFIG=config.toml pasejo identity add --store non-existing --file some-identity
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```

```
$ PASEJO_CONFIG=config.toml pasejo identity add --store default --global --file other-identity
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo identity add --file <FILE> --store <STORE>

For more information, try '--help'.

```
