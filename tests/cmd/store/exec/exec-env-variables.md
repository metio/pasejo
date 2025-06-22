```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store exec -- printenv PASEJO_EXEC_STORE_PATH
[CWD]/store

$ pasejo store exec -- printenv PASEJO_EXEC_STORE_PARENT
[CWD]

$ pasejo store exec -- printenv PASEJO_EXEC_COMMAND
printenv

```
