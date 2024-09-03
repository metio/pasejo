```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --alias default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --alias other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo store remove --alias default
Store default removed

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/somewhere --alias some-name --default
Store initialized at [CWD]/path/to/somewhere
Store some-name is now the default

$ PASEJO_CONFIG=config.toml pasejo store remove --alias some-name
Store some-name removed

```
