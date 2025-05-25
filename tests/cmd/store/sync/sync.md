```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo store sync --verbose

$ pasejo store sync --pull --verbose
debug: Pulling changes from remote for store 'something'

$ pasejo store sync --push --verbose
debug: Pushing changes to remote for store 'something'

$ pasejo store sync --pull --push --verbose
debug: Pulling changes from remote for store 'something'
debug: Pushing changes to remote for store 'something'

$ pasejo store sync --pull=false --push=true --verbose 
debug: Pushing changes to remote for store 'something'

$ pasejo store sync --pull=true --push=false --verbose 
debug: Pulling changes from remote for store 'something'

$ pasejo store sync --pull true --push false --verbose 
debug: Pulling changes from remote for store 'something'

$ pasejo store sync --pull false --push true --verbose 
debug: Pushing changes to remote for store 'something'

```
