```
$ pasejo store add --path ancestor --name ancestor
Store 'ancestor' added at '[CWD]/ancestor'

$ pasejo store add --path current --name current
Store 'current' added at '[CWD]/current'

$ pasejo store add --path other --name other
Store 'other' added at '[CWD]/other'

$ pasejo identity add --file some-identity --store ancestor
Identity using file '[CWD]/some-identity' added

$ pasejo identity add --file some-identity --store current
Identity using file '[CWD]/some-identity' added

$ pasejo identity add --file some-identity --store other
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --store ancestor
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --store current
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --store other
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --type totp --secret JFQDYMZAN5YGK3RAONXXK4TDMU some-name --store current
Added one-time password at 'some-name'

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU some-name --store other
Added one-time password at 'some-name'

$ pasejo store decrypt --store ancestor
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]

[otp]


$ pasejo store merge --common-ancestor ancestor --current-version current --other-version other
? 1
Merge conflict for one-time password at 'some-name': values differ in the two versions
Error: Merge conflict detected in one-time passwords. Please resolve manually.

$ pasejo store decrypt --store ancestor
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]

[otp]


```
