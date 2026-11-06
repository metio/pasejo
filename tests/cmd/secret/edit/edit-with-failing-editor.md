```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate some-secret
Generated secret at 'some-secret'

$ pasejo secret edit some-secret
? 1
...
Error: Could not read secret for some-secret

Caused by:
    The editor 'false' did not exit successfully, so the secret was left unchanged

$ pasejo secret edit missing
? 1
Error: Secret does not exist at missing. Use 'pasejo secret add' to create it.

```
