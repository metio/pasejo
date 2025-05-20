```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
info: Inserted one-time password at 'some-name'

$ PASEJO_CONFIG=config.toml pasejo otp show some-name
266507

$ PASEJO_CONFIG=config.toml pasejo otp show some-name --quiet
290697

$ PASEJO_CONFIG=config.toml pasejo otp show some-name --verbose
debug: Pulling changes from remote for store 'something'
debug: Showing one-time password at 'some-name'
861574

```
