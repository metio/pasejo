```console
$ PASEJO_CONFIG=config.toml pasejo store init --path store --name default --vcs none
info: Store initialized at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default --public-key "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"
info: Recipient for 'age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default  --public-key "age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg" --name 'Alice Asounder'
info: Recipient for 'age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --store default  --public-key "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p" --name 'Bob Builder'
info: Recipient for 'age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p' updated

```

```console
$ PASEJO_CONFIG=config.toml pasejo recipient add --store non-existing --public-key "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"
? 2
error: invalid value 'non-existing' for '--store <STORE>': Store with name 'non-existing' does not exist in configuration

For more information, try '--help'.

```
