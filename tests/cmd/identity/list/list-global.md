```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --global
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo identity list --global
[CWD]/some-identity (global)

```
