```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added
There are no identities in the store 'something' yet. Please add one using 'pasejo identity add ...'

$ pasejo recipient add --public-key age1acvmq9pv8lwr60xa7va36vnz2wr78zutk3mpy3jzyw8pfpdeg45qjhfqp8
? 1
Error: Cannot decrypt store

Caused by:
    No matching keys found

```
