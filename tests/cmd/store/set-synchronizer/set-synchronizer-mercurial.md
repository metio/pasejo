```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store set-synchronizer mercurial
Store 'something' now synchronizes with Mercurial

$ pasejo store set-synchronizer hg
? 2
error: invalid value 'hg' for '<SYNCHRONIZER>'
  [possible values: none, git, mercurial, pijul]

For more information, try '--help'.

```
