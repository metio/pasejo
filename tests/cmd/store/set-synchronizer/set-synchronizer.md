```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store set-synchronizer git
Store 'something' now synchronizes with Git

$ pasejo store set-synchronizer Git
? 2
error: invalid value 'Git' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

  tip: a similar value exists: 'git'

For more information, try '--help'.

$ pasejo store set-synchronizer GIT
? 2
error: invalid value 'GIT' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

For more information, try '--help'.

```
