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

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: Mindestens eine Zeichenart muss aktiviert sein

$ pasejo secret generate edited
[..]

$ pasejo secret edit edited
? 1
...
Error: Konnte Geheimnis für edited nicht lesen

Caused by:
    Der Editor 'false' wurde nicht erfolgreich beendet, daher bleibt das Geheimnis unverändert

```
