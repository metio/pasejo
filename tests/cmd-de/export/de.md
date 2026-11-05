```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo otp add --type hotp --secret JEQDYMZAN5YGK3RAONXXK4TDMU bank
[..]

$ pasejo otp add --secret JEQDYMZAN5YGK3RAONXXK4TDMU orphan
[..]

$ pasejo export bitwarden --out backup.json
Warnung: Der Export enthält jedes Geheimnis im Klartext.
Warnung: Bitwarden kann keine HOTP-Einmalpasswörter speichern, daher sind diese nicht Teil des Exports: bank
Warnung: Diese Einmalpasswörter haben kein gleichnamiges Geheimnis und sind nicht Teil des Exports: orphan
Export nach 'backup.json' geschrieben

```
