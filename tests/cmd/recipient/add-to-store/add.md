```console
$ PASEJO_CONFIG=config.toml pasejo store init --path store --name default
Store initialized at [CWD]/store

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default --public-key "age12abcdefghijklmnopqrstuvwxyz"
Recipient added

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default  --public-key "age12zyxwvutsrqponmlkjihgfedcba" --name 'Alice Asounder'
Recipient added

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default  --public-key "age12abcdefghijklmnopqrstuvwxyz" --name 'Bob Builder'
Recipient updated

```

```console
$ PASEJO_CONFIG=config.toml pasejo recipient add --store non-existing --public-key "age12abcdefghijklmnopqrstuvwxyz"
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```
