```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
info: Inserted one-time password at 'some-name'

$ PASEJO_CONFIG=config.toml pasejo otp add --url otpauth://totp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example sub/name
info: Inserted one-time password at 'sub/name'

$ PASEJO_CONFIG=config.toml pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU unique-name
info: Inserted one-time password at 'unique-name'

$ PASEJO_CONFIG=config.toml pasejo otp list
some-name
sub/name
unique-name

```
