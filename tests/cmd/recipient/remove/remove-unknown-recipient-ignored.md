```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --ignore-unknown
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' does not exist in store - ignoring

$ PASEJO_CONFIG=config.toml pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --ignore-unknown --quiet

$ PASEJO_CONFIG=config.toml pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --ignore-unknown --verbose
debug: Pulling changes from remote for store 'something'
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' does not exist in store - ignoring

```
