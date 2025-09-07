```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo hook set --push 'echo push' --pull 'echo pull'

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU unique-name
Added one-time password at 'unique-name'

$ pasejo otp copy unique-name new-name --verbose
Executing pull hooks for store 'something'
Copied one-time password from 'unique-name' to 'new-name'
Executing push hooks for store 'something'

```
