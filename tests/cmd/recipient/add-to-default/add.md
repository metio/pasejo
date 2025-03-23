```console
$ PASEJO_CONFIG=config.toml pasejo store init --path default --name default --vcs none
info: Store initialized at '[CWD]/default'

$ PASEJO_CONFIG=config.toml pasejo store init --path other --name other --vcs none
info: Store initialized at '[CWD]/other'

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"
info: Recipient for 'age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p' added

$ PASEJO_CONFIG=config.toml pasejo store set-default default
info: Store 'default' is now the default

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg" --name 'Alice Asounder'
info: Recipient for 'age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key "age16jsuwahftuk2qquvpnykqwzgsygrknyme2vdm23stpk9pq0w4s7sh8lugy" --name 'Bob Builder'
info: Recipient for 'age16jsuwahftuk2qquvpnykqwzgsygrknyme2vdm23stpk9pq0w4s7sh8lugy' added

$ PASEJO_CONFIG=config.toml PASEJO_DEFAULT_STORE=other pasejo recipient add --public-key "age1demvgx6ckfgc3qassr847gznfhkalvkd3vsf0j8tc0c7swy7p9rs2chev2" --name 'Bob Builder'
info: Recipient for 'age1demvgx6ckfgc3qassr847gznfhkalvkd3vsf0j8tc0c7swy7p9rs2chev2' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --file public-key
info: Recipient for 'age1lgxhnuecad3k7hzt9em7wa9ryhwle7vctq5cd85pvrqz48lkzcrqpvkr3e' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --github sebhoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOycTzXsl1jetsf+Ezi/2FCVU8oydXxxJhb9D7n6MlGf' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --gitlab sebastian.hoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr Sebastian Hoß (gitlab.com)' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --codeberg sebhoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB' added

```