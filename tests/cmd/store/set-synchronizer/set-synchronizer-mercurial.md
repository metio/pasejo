```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store set-synchronizer mercurial
info: Store 'something' now synchronizes with Mercurial

$ PASEJO_CONFIG=config.toml pasejo store set-synchronizer hg
? 2
error: invalid value 'hg' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

For more information, try '--help'.

```
