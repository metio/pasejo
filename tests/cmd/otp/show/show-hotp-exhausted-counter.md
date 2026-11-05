A HOTP entry whose counter cannot advance refuses to emit a code, since the
code could not be followed by a counter increment and would be replayed.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo otp add --type hotp --secret JEQDYMZAN5YGK3RAONXXK4TDMU --counter 18446744073709551615 exhausted
Added one-time password at 'exhausted'

$ pasejo otp show exhausted
? 1
Error: The HOTP counter has reached its maximum value and cannot advance

```
