```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo store exec -- echo hi
hi

$ pasejo store exec -- echo "\$PASEJO_EXEC_STORE_PARENT"
$PASEJO_EXEC_STORE_PARENT

$ pasejo store exec -- echo '$PASEJO_EXEC_STORE_PARENT'
$PASEJO_EXEC_STORE_PARENT

```
