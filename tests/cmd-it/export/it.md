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
Attenzione: l'esportazione contiene ogni segreto in chiaro.
Attenzione: Bitwarden non può memorizzare password monouso HOTP, quindi queste non fanno parte dell'esportazione: bank
Attenzione: queste password monouso non hanno un segreto con lo stesso nome e non fanno parte dell'esportazione: orphan
Esportazione scritta in 'backup.json'

```
