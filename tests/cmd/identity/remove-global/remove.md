```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
Identity added

$ PASEJO_CONFIG=config.toml pasejo identity remove --file some-identity --global
Identity removed

```

```
$ PASEJO_CONFIG=config.toml pasejo identity remove --file non-existing --global
? 2
error: invalid value 'non-existing' for '--file <FILE>': file does not match any known identity

Usage: pasejo identity remove [OPTIONS] --file <FILE>

For more information, try '--help'.

$ PASEJO_CONFIG=config.toml pasejo identity remove --file non-existing --global --ignore-missing

```
