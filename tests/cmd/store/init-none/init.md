```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --alias default
Store initialized at [CWD]/path/to/store

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/another --alias another --default
Store initialized at [CWD]/path/to/another
Store another is now the default

```

```console
$ PASEJO_CONFIG=config.toml pasejo store init --path existing-store --alias existing
Store initialized at [CWD]/existing-store

```
