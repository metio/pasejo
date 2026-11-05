```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: パスワードの長さを 0 にすることはできません

$ pasejo secret generate --length 2 short
? 1
Error: パスワードの長さが必要な文字の種類に対して短すぎます

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: 少なくとも 1 種類の文字を有効にする必要があります

```
