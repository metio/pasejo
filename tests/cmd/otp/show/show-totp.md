```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://totp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example some-name
info: Added one-time password at 'some-name'

$ pasejo otp show some-name
558961

$ pasejo otp show some-name --quiet
266507

$ pasejo otp show some-name --verbose
debug: Pulling changes from remote for store 'something'
debug: Showing one-time password at 'some-name'
290697

```
