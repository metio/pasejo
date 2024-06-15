```
$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
? 2
error: invalid value 'some-identity' for '--file <FILE>': file was already added as an identity

Usage: pasejo identity add [OPTIONS] --file <FILE>

For more information, try '--help'.

```
