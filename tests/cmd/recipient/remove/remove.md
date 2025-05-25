```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' added

$ pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' removed

$ pasejo recipient add --public-key age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' added

$ pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --quiet

$ pasejo recipient add --public-key age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' added

$ pasejo recipient remove age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --verbose
debug: Pulling changes from remote for store 'something'
debug: Pushing changes to remote for store 'something'
info: Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' removed

```
