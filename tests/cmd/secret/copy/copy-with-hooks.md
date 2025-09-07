```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo hook set --push 'echo push' --pull 'echo pull'

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate some-secret
Generated secret at 'some-secret'

$ pasejo secret copy some-secret somewhere-else --verbose
Executing pull hooks for store 'something'
Copied secret from 'some-secret' to 'somewhere-else'
Executing push hooks for store 'something'

$ pasejo secret list
some-secret
somewhere-else

```
