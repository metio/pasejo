```
$ pasejo store add --path store --name personal
Store 'personal' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store personal
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate sites/example.com/alice
Generated secret at 'sites/example.com/alice'

$ pasejo browser query example.com
{"results":[{"name":"sites/example.com/alice","store":"personal","username":"","password":"[..]","url":null,"match_kind":"name","score":0.5,"fields":{}}]}

```
