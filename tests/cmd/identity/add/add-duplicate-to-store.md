```
$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
? 2
error: invalid value 'some-identity' for '--file <FILE>': file was already added as an identity

Usage: pasejo identity add [OPTIONS] --file <FILE>

For more information, try '--help'.

```
