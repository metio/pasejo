```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo recipient add --codeberg sebhoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB' added
warn: There are no identities in the store 'something' yet. Please add one using 'pasejo identity add ...'

```
