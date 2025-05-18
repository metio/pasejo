```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo store set-synchronizer git
info: Store 'something' now synchronizes with Git

$ PASEJO_CONFIG=config.toml pasejo store set-synchronizer Git
? 2
error: invalid value 'Git' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

  tip: a similar value exists: 'git'

For more information, try '--help'.

$ PASEJO_CONFIG=config.toml pasejo store set-synchronizer GIT
? 2
error: invalid value 'GIT' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

For more information, try '--help'.

```
