```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default --default --vcs none
info: Store initialized at '[CWD]/path/to/store'
info: Store 'default' is now the default

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --name other --vcs none
info: Store initialized at '[CWD]/path/to/other'

$ PASEJO_CONFIG=config.toml pasejo identity add --store default --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity add --store other --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity add --file other-identity
info: Identity using file '[CWD]/other-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity remove --store default --file some-identity
info: Identity using file '[CWD]/some-identity' removed

$ PASEJO_CONFIG=config.toml pasejo identity remove --store other --file some-identity
info: Identity using file '[CWD]/some-identity' removed

$ PASEJO_CONFIG=config.toml pasejo identity remove --file other-identity
info: Identity using file '[CWD]/other-identity' removed

```

```
$ PASEJO_CONFIG=config.toml pasejo identity remove --file non-existing
? 2
error: invalid value 'non-existing' for '--file <FILE>': file does not match any known identity

Usage: pasejo identity remove [OPTIONS] --file <FILE>

For more information, try '--help'.

$ PASEJO_CONFIG=config.toml pasejo identity remove --file non-existing --ignore-missing

```

```
$ PASEJO_CONFIG=config.toml pasejo identity remove --store non-existing --file some-identity
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```

```
$ PASEJO_CONFIG=config.toml pasejo identity remove --store default --global --file some-identity
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo identity remove --file <FILE> --store <STORE>

For more information, try '--help'.

```
