```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store add --path another --name another
Store 'another' added at '[CWD]/another'

$ pasejo hook run --verbose

$ pasejo hook run --pull --verbose

$ pasejo hook run --push --verbose

$ pasejo hook run --pull --push --verbose

$ pasejo hook run --pull=false --push=true --verbose 

$ pasejo hook run --pull=true --push=false --verbose 

$ pasejo hook run --pull true --push false --verbose 

$ pasejo hook run --pull false --push true --verbose 

$ pasejo hook run --all --pull --verbose

$ pasejo hook run --all --pull --push --verbose

$ pasejo hook run --all --store something
? 2
error: the argument '--all [<ALL>]' cannot be used with '--store <STORE>'

Usage: pasejo hook run --all [<ALL>]

For more information, try '--help'.

$ pasejo hook set --push 'echo push' --pull 'echo pull'

$ pasejo hook run --verbose

$ pasejo hook set --pull 'sdfgdf'

$ pasejo hook run --pull --verbose
? 1
Executing pull hooks for store 'something'
Error: Failed to run command

Caused by:
    No such file or directory (os error 2)

$ pasejo hook set --push 'sdfgdf'

$ pasejo hook run --push --verbose
? 1
Executing push hooks for store 'something'
Error: Failed to run command

Caused by:
    No such file or directory (os error 2)

```
