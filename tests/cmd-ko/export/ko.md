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
경고: 내보내기에는 모든 비밀이 평문으로 포함됩니다.
경고: Bitwarden은 HOTP 일회용 비밀번호를 저장할 수 없으므로 다음 항목은 내보내기에 포함되지 않습니다: bank
경고: 다음 일회용 비밀번호에는 같은 이름의 비밀이 없어 내보내기에 포함되지 않습니다: orphan
내보내기를 'backup.json'에 작성했습니다

```
