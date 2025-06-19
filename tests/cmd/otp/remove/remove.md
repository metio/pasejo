```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
info: Added one-time password at 'some-name'

$ pasejo otp add --url otpauth://totp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example sub/name
info: Added one-time password at 'sub/name'

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU unique-name
info: Added one-time password at 'unique-name'

$ pasejo otp remove unique-name --force --quiet

$ pasejo otp remove sub/name --force --verbose
debug: Pulling changes from remote for store 'something'
debug: Pushing changes to remote for store 'something'

$ pasejo otp remove some-name --force

```
