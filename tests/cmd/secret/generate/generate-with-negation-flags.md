```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate no-kinds --no-numbers --no-lowercase-letters --no-uppercase-letters
? 1
Error: At least one kind of character must be enabled

$ pasejo secret generate numbers-again --no-numbers --no-lowercase-letters --no-uppercase-letters --numbers --length 5
Generated secret at 'numbers-again'

$ pasejo secret generate lax --length 2 --no-strict
Generated secret at 'lax'

$ pasejo secret generate strict-again --length 2 --no-strict --strict
? 1
Error: The password length is too short for the required character kinds

$ pasejo secret generate similar --length 5 --no-exclude-similar-characters
Generated secret at 'similar'

$ pasejo secret list
lax
numbers-again
similar

```
