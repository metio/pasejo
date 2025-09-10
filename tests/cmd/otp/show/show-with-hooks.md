```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo hook set --push 'echo push' --pull 'echo pull'

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-hotp
Added one-time password at 'some-hotp'

$ pasejo otp add --url otpauth://totp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example some-totp
Added one-time password at 'some-totp'

$ pasejo otp show some-hotp --verbose
Executing pull hooks for store 'something'
Showing one-time password at 'some-hotp'
[..]
Executing push hooks for store 'something'

$ pasejo otp show some-totp --verbose
Executing pull hooks for store 'something'
Showing one-time password at 'some-totp'
[..]

```
