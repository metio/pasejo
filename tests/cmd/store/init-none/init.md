```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name default
Store initialized at [CWD]/path/to/store

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/another --name another --default
Store initialized at [CWD]/path/to/another
Store another is now the default

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path existing-store --name existing
Store initialized at [CWD]/existing-store

```
