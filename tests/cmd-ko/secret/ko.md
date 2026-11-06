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

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: 최소 한 종류의 문자를 활성화해야 합니다

$ pasejo secret generate edited
[..]

$ pasejo secret edit edited
? 1
...
Error: edited의 비밀을 읽을 수 없습니다

Caused by:
    편집기 'false'이(가) 정상적으로 종료되지 않아 비밀이 변경되지 않았습니다

```
