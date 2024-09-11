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

$ PASEJO_CONFIG=config.toml PASEJO_DEFAULT_STORE_NAME=other pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
info: Recipient for 'age12abcdefghijklmnopqrstuvwxyz' added

```