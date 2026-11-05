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
警告: エクスポートにはすべてのシークレットが平文で含まれます。
警告: Bitwarden は HOTP ワンタイムパスワードを保存できないため、次のものはエクスポートに含まれません: bank
警告: 次のワンタイムパスワードには同名のシークレットがないため、エクスポートに含まれません: orphan
エクスポートを 'backup.json' に書き込みました

```
