```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name something --vcs none
info: Store initialized at '[CWD]/path/to/store'

$ mkdir -p path/to/store/sub/folder
$ touch path/to/store/secret-in-root.age
$ touch path/to/store/sub/secret-name1.age
$ touch path/to/store/sub/folder/secret-name2.age

$ PASEJO_CONFIG=config.toml pasejo secret list
something/secret-in-root
something/sub/folder/secret-name2
something/sub/secret-name1

$ PASEJO_CONFIG=config.toml pasejo secret move secret-in-root moved-secret-in-root

$ PASEJO_CONFIG=config.toml pasejo secret list
something/moved-secret-in-root
something/sub/folder/secret-name2
something/sub/secret-name1

$ PASEJO_CONFIG=config.toml pasejo secret move sub/folder/secret-name2 sub/moved-here

$ PASEJO_CONFIG=config.toml pasejo secret list
something/moved-secret-in-root
something/sub/moved-here
something/sub/secret-name1

$ PASEJO_CONFIG=config.toml pasejo secret move sub/secret-name1 sub/newly-created-folder/moved

$ PASEJO_CONFIG=config.toml pasejo secret list
something/moved-secret-in-root
something/sub/moved-here
something/sub/newly-created-folder/moved

```
