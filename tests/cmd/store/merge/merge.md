```
$ pasejo store add --path something --name something
Store 'something' added at '[CWD]/something'

$ pasejo store add --path other --name other
Store 'other' added at '[CWD]/other'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo identity add --file some-identity --store other
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --store something
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd --store other
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add secret1 --store something
Added secret at 'secret1'

$ pasejo secret add secret1 --store other
Added secret at 'secret1'

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU some-name --store something
Added one-time password at 'some-name'

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU some-name --store other
Added one-time password at 'some-name'

$ pasejo store decrypt --store something
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]
secret1 = ""

[otp.some-name]
secret = "JEQDYMZAN5YGK3RAONXXK4TDMU"
otp_type = "Totp"
algorithm = "Sha1"
digits = 6
period = 30
counter = 1
skew = 0


$ pasejo store merge --common-ancestor something --current-version something --other-version other

$ pasejo store decrypt --store something
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]
secret1 = ""

[otp.some-name]
secret = "JEQDYMZAN5YGK3RAONXXK4TDMU"
otp_type = "Totp"
algorithm = "Sha1"
digits = 6
period = 30
counter = 1
skew = 0


```
