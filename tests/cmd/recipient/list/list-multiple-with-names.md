```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --name 'Bob Builder'
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04 --name 'Alice Asounder'
Recipient for 'age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04' added

$ pasejo recipient list
# Bob Builder
age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
# Alice Asounder
age1h0fd3l7c99kruc5n2h488vpgm4vxa4s7tca5u5ltv9lcjfxr6cfqw9lf04

```
