```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
Added one-time password at 'some-name'

$ pasejo otp show some-name
266507

$ pasejo otp show some-name --quiet
290697

$ pasejo otp show some-name --verbose
Pulling changes from remote for store 'something'
Showing one-time password at 'some-name'
861574

```
