```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
Identity added

```

```
$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
? 2
error: invalid value 'some-identity' for '--file <FILE>': file was already added as an identity

Usage: pasejo identity add [OPTIONS] --file <FILE>

For more information, try '--help'.

```
