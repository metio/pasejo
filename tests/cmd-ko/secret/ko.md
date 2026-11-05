```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: 비밀번호 길이는 0일 수 없습니다

$ pasejo secret generate --length 2 short
? 1
Error: 비밀번호 길이가 필요한 문자 종류에 비해 너무 짧습니다

```
