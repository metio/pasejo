```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add some-secret
Added secret at 'some-secret'

$ pasejo secret show some-secret --line 1 --skip-lines 1
? 2
error: the argument '--line <LINE>' cannot be used with '--skip-lines <SKIP_LINES>'

Usage: pasejo secret show --line <LINE> <SECRET_PATH>

For more information, try '--help'.

$ pasejo secret show some-secret --skip-lines 0
? 2
error: invalid value '0' for '--skip-lines <SKIP_LINES>': Count must not be 0. Use 1 to skip the first line

For more information, try '--help'.

$ pasejo secret show some-secret --line abc
? 2
error: invalid value 'abc' for '--line <LINE>': 'abc' is not a valid line number

For more information, try '--help'.

```
