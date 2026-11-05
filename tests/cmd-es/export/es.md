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
Advertencia: la exportación contiene todos los secretos en texto plano.
Advertencia: Bitwarden no puede almacenar contraseñas de un solo uso HOTP, por lo que estas no forman parte de la exportación: bank
Advertencia: estas contraseñas de un solo uso no tienen un secreto con el mismo nombre y no forman parte de la exportación: orphan
Exportación escrita en 'backup.json'

```
