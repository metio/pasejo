`--out <FILE>` writes the export to a file created mode 0600 (instead of
stdout), so the cleartext export does not land at the umask default via a
shell redirection nor sit in shell history. The plaintext warning still
fires on stderr. An empty store is used so the output is deterministic.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo export bitwarden --pretty --out backup.json
Warning: the export contains every secret in plaintext.
Wrote the export to 'backup.json'

```
