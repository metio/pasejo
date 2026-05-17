```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add some-secret
Added secret at 'some-secret'

$ pasejo secret show some-secret


$ pasejo secret show some-secret --line 1


$ pasejo secret show some-secret --line 0
? 2
error: invalid value '0' for '--line <LINE>': Line number must not be 0. Use 1 for the first line, -1 for the last

For more information, try '--help'.

```
