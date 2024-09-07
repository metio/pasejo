```console
$ PASEJO_CONFIG=config.toml pasejo store init --path default --name default
Store initialized at [CWD]/default

$ PASEJO_CONFIG=config.toml pasejo store init --path other --name other
Store initialized at [CWD]/other

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz"
Recipient added

$ PASEJO_CONFIG=config.toml pasejo store set-default default
Store default is now the default

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12zyxwvutsrqponmlkjihgfedcba" --name 'Alice Asounder'
Recipient added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
Recipient updated

$ PASEJO_CONFIG=config.toml PASEJO_DEFAULT_STORE_NAME=other pasejo recipient add --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
Recipient added

```