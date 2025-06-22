```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add secretA
info: Added secret at 'secretA'

$ pasejo secret add secretB
info: Added secret at 'secretB'

$ pasejo secret add secretC
info: Added secret at 'secretC'

$ pasejo secret add secretD
info: Added secret at 'secretD'

$ pasejo store decrypt --store-path store
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]
secretA = ""
secretB = ""
secretC = ""
secretD = ""

[otp]


```
