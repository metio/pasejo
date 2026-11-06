```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: 密码长度不能为 0

$ pasejo secret generate --length 2 short
? 1
Error: 密码长度对于所需的字符类型来说太短

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: 必须至少启用一种字符类型

$ pasejo secret generate edited
[..]

$ pasejo secret edit edited
? 1
...
Error: 无法读取 edited 的密钥

Caused by:
    编辑器 'false' 未成功退出，因此密钥保持不变

```
