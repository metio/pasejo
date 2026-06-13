Querying a store name that does not exist reports the typo'd name
distinctly, instead of the misleading "no store configured / run
`pasejo store add`" message that fires only when there are genuinely no
stores at all.

```
$ pasejo store add --path store --name personal
Store 'personal' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store personal
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo browser query --store ghost example.com
? 2
error: invalid value 'ghost' for '--store <STORE>': Store with name 'ghost' does not exist in configuration

For more information, try '--help'.

```
