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
Avertissement : l'export contient tous les secrets en clair.
Avertissement : Bitwarden ne peut pas stocker de mots de passe à usage unique HOTP, ceux-ci ne font donc pas partie de l'export : bank
Avertissement : ces mots de passe à usage unique n'ont pas de secret du même nom et ne font pas partie de l'export : orphan
Export écrit dans 'backup.json'

```
