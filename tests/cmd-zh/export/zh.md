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
警告：导出内容包含所有明文密钥。
警告：Bitwarden 无法存储 HOTP 一次性密码，因此以下条目不包含在导出中：bank
警告：以下一次性密码没有同名的密文，不包含在导出中：orphan
已将导出写入 'backup.json'

```
