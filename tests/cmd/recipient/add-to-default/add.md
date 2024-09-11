```console
$ PASEJO_CONFIG=config.toml pasejo store init --path default --name default
info: Store initialized at '[CWD]/default'

$ PASEJO_CONFIG=config.toml pasejo store init --path other --name other
info: Store initialized at '[CWD]/other'

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz"
info: Recipient for 'age12abcdefghijklmnopqrstuvwxyz' added

$ PASEJO_CONFIG=config.toml pasejo store set-default default
info: Store 'default' is now the default

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12zyxwvutsrqponmlkjihgfedcba" --name 'Alice Asounder'
info: Recipient for 'age12zyxwvutsrqponmlkjihgfedcba' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
info: Recipient for 'age12abcdefghijklmnopqrstuvwxyz' updated

$ PASEJO_CONFIG=config.toml PASEJO_DEFAULT_STORE=other pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
info: Recipient for 'age12abcdefghijklmnopqrstuvwxyz' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --file public-key
info: Recipient for 'age12abcdefghxxxmnopqrstuvwxyz' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --github sebhoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOycTzXsl1jetsf+Ezi/2FCVU8oydXxxJhb9D7n6MlGf' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --gitlab sebastian.hoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr Sebastian Hoß (gitlab.com)' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --codeberg sebhoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB' added

```