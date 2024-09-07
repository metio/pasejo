```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default
Store initialized at [CWD]/path/to/store

$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/other --name other
Store initialized at [CWD]/path/to/other

$ PASEJO_CONFIG=config.toml pasejo store remove --name default
Store default removed

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/somewhere --name some-name --default
Store initialized at [CWD]/path/to/somewhere
Store some-name is now the default

$ PASEJO_CONFIG=config.toml pasejo store remove --name some-name
Store some-name removed

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/someplace --name some-place
Store initialized at [CWD]/path/to/someplace

$ PASEJO_CONFIG=config.toml pasejo store remove --name some-place
Store some-place removed

$ PASEJO_CONFIG=config.toml pasejo store remove --name some-place
? 2
error: invalid value 'some-place' for '--name <NAME>': Store with name 'some-place' does not exist in configuration

For more information, try '--help'.

```
