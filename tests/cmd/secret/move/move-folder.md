```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add some/secrets/here
Added secret at 'some/secrets/here'

$ pasejo secret add some/secrets/other
Added secret at 'some/secrets/other'

$ pasejo secret list
some/secrets/here
some/secrets/other

$ pasejo secret move some/secrets somewhere/else
? 1
Error: No secret found at 'some/secrets'

$ pasejo secret list
some/secrets/here
some/secrets/other

```
