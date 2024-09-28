```console
$ PASEJO_CONFIG=config.toml pasejo store init --path path/to/store --name something --vcs none
info: Store initialized at '[CWD]/path/to/store'

$ mkdir -p path/to/store/sub/folder/name
$ mkdir -p path/to/store/sub/folder/dir
$ mkdir -p path/to/store/sub/sub/folder
$ mkdir -p path/to/store/sub/sub/name
$ mkdir -p path/to/store/another/name/here
$ touch path/to/store/secret-in-root.age
$ touch path/to/store/other-in-root.age
$ touch path/to/store/sub/secret-name1.age
$ touch path/to/store/sub/folder/secret-name2.age
$ touch path/to/store/sub/folder/name/secret-name3.age
$ touch path/to/store/sub/folder/dir/secret-name4.age
$ touch path/to/store/sub/sub/folder/secret-name5.age
$ touch path/to/store/sub/sub/name/secret-name6.age
$ touch path/to/store/another/name/here/secret-name7.age
$ touch path/to/store/another/name/here/secret-name8.age

$ PASEJO_CONFIG=config.toml pasejo secret list --tree
something
├── another
│   └── name
│       └── here
│           ├── secret-name7
│           └── secret-name8
├── other-in-root
├── secret-in-root
└── sub
    ├── folder
    │   ├── dir
    │   │   └── secret-name4
    │   ├── name
    │   │   └── secret-name3
    │   └── secret-name2
    ├── secret-name1
    └── sub
        ├── folder
        │   └── secret-name5
        └── name
            └── secret-name6

$ PASEJO_CONFIG=config.toml pasejo secret list
something/another/name/here/secret-name7
something/another/name/here/secret-name8
something/other-in-root
something/secret-in-root
something/sub/folder/dir/secret-name4
something/sub/folder/name/secret-name3
something/sub/folder/secret-name2
something/sub/secret-name1
something/sub/sub/folder/secret-name5
something/sub/sub/name/secret-name6

```
