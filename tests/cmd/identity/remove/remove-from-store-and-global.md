```
$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity remove --store something --global --file some-identity
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo identity remove --file <FILE> --store <STORE>

For more information, try '--help'.

```
