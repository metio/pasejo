```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store add --path another --name another
Store 'another' added at '[CWD]/another'

$ pasejo store sync --verbose

$ pasejo store sync --pull --verbose
Pulling changes from remote for store 'something'

$ pasejo store sync --push --verbose
Pushing changes to remote for store 'something'

$ pasejo store sync --pull --push --verbose
Pulling changes from remote for store 'something'
Pushing changes to remote for store 'something'

$ pasejo store sync --pull=false --push=true --verbose 
Pushing changes to remote for store 'something'

$ pasejo store sync --pull=true --push=false --verbose 
Pulling changes from remote for store 'something'

$ pasejo store sync --pull true --push false --verbose 
Pulling changes from remote for store 'something'

$ pasejo store sync --pull false --push true --verbose 
Pushing changes to remote for store 'something'

$ pasejo store sync --all --pull --verbose
Pulling changes from remote for store 'something'
Pulling changes from remote for store 'another'

$ pasejo store sync --all --pull --push --verbose
Pulling changes from remote for store 'something'
Pushing changes to remote for store 'something'
Pulling changes from remote for store 'another'
Pushing changes to remote for store 'another'

$ pasejo store sync --all --store something
? 2
error: the argument '--all [<ALL>]' cannot be used with '--store <STORE>'

Usage: pasejo store sync --all [<ALL>]

For more information, try '--help'.

```
