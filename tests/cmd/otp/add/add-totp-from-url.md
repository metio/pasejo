```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://totp/Example:alice@example.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example some-name
Added one-time password at 'some-name'

$ pasejo otp add --url otpauth://totp/Example:alice@example.com?secret=jeqdymzan5ygk3raonxxk4tdmu&issuer=Example lower-case-secret
Added one-time password at 'lower-case-secret'

$ pasejo otp add --url otpauth://totp/Different:alice@example.com?secret=jeqdymzan5ygk3raonxxk4tdmu&issuer=Example mismatched-label-issuer
Added one-time password at 'mismatched-label-issuer'

$ pasejo otp add --url otpauth://totp/Different:alice@example.com?secret=jeqdymztebxxazlo&issuer=Example short-secret
Added one-time password at 'short-secret'

```
