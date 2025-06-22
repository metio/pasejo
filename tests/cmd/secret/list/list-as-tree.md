```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret add some-secret
Added secret at 'some-secret'

$ pasejo secret add other-secret
Added secret at 'other-secret'

$ pasejo secret add nested/secret
Added secret at 'nested/secret'

$ pasejo secret generate sub/sub/sub/secret
Generated secret at 'sub/sub/sub/secret'

$ pasejo secret list --tree

├── nested
│   └── secret
├── other-secret
├── some-secret
└── sub
    └── sub
        └── sub
            └── secret

```
