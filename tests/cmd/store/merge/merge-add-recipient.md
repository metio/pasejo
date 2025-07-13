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

$ pasejo recipient add --public-key age1acvmq9pv8lwr60xa7va36vnz2wr78zutk3mpy3jzyw8pfpdeg45qjhfqp8 --store other
Recipient for 'age1acvmq9pv8lwr60xa7va36vnz2wr78zutk3mpy3jzyw8pfpdeg45qjhfqp8' added

$ pasejo store decrypt --store something
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]

[otp]


$ pasejo store merge --common-ancestor something --current-version something --other-version other

$ pasejo store decrypt --store something
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[[recipients]]
name = ""
public_key = "age1acvmq9pv8lwr60xa7va36vnz2wr78zutk3mpy3jzyw8pfpdeg45qjhfqp8"

[secrets]

[otp]


```
