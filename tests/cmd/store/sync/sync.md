```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

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

```
