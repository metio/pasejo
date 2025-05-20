```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo secret add secret1
info: Inserted secret at 'secret1'

$ PASEJO_CONFIG=config.toml pasejo secret add secret2
info: Inserted secret at 'secret2'

$ PASEJO_CONFIG=config.toml pasejo secret add secret3
info: Inserted secret at 'secret3'

$ PASEJO_CONFIG=config.toml pasejo secret add secret4
info: Inserted secret at 'secret4'

$ PASEJO_CONFIG=config.toml pasejo store decrypt
[[recipients]]
name = ""
public_key = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd"

[secrets]
secret1 = ""
secret2 = ""
secret3 = ""
secret4 = ""

[otp]


```
