```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --url otpauth://hotp/Example:alice@google.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
Added one-time password at 'some-name'

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU unique-name
Added one-time password at 'unique-name'

$ pasejo otp copy unique-name new-name
Copied one-time password from 'unique-name' to 'new-name'

$ pasejo otp copy some-name deeply/nested/name/for/password --quiet

$ pasejo otp copy deeply/nested/name/for/password here-it-is --verbose
Copied one-time password from 'deeply/nested/name/for/password' to 'here-it-is'

$ pasejo otp show here-it-is
[..]

```
