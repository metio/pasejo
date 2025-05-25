```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo store set-synchronizer pijul --store non-existing
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```
