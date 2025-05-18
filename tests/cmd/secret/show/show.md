```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo secret add some-secret
info: Inserted secret at 'some-secret'

$ PASEJO_CONFIG=config.toml pasejo secret show some-secret


$ PASEJO_CONFIG=config.toml pasejo secret show some-secret --line 0


```
