```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1acvmq9pv8lwr60xa7va36vnz2wr78zutk3mpy3jzyw8pfpdeg45qjhfqp8 --quiet

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age142fqusnk9ye2nap204yryenxejkwuztt0xqnt8626gx7c5a96djs7smpyg --verbose
debug: Pulling changes from remote for store 'something'
info: Recipient for 'age142fqusnk9ye2nap204yryenxejkwuztt0xqnt8626gx7c5a96djs7smpyg' added
debug: Pushing changes to remote for store 'something'

```
