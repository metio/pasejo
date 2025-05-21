```
$ PASEJO_CONFIG=config.toml pasejo config set clipboard-timeout 123

$ PASEJO_CONFIG=config.toml pasejo config get clipboard-timeout
123

$ PASEJO_CONFIG=config.toml pasejo config set clipboard-timeout 1

$ PASEJO_CONFIG=config.toml pasejo config get clipboard-timeout
1

$ PASEJO_CONFIG=config.toml pasejo config set clipboard-timeout -- -1
? 1
Error: invalid digit found in string

$ PASEJO_CONFIG=config.toml pasejo config get clipboard-timeout
1

```
