```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo secret copy some-secret somewhere-else --store another
? 2
error: invalid value 'another' for '--store <STORE>': Store with name 'another' does not exist in configuration

For more information, try '--help'.

```
