```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: Die Passwortlänge darf nicht 0 sein

$ pasejo secret generate --length 2 short
? 1
Error: Die Passwortlänge ist zu kurz für die geforderten Zeichenarten

```
