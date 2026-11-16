```
$ pasejo recipient list
? 1
Error: 存储 'team' 是远程存储；接收者、钩子和合并不适用于它。

$ pasejo hook get
? 1
Error: 存储 'team' 是远程存储；接收者、钩子和合并不适用于它。

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' 不是 YYYY-MM-DD 格式的日期。

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: 存储 'files' 不是远程存储。

$ pasejo org devices --org nope
? 1
Error: 没有已注册的组织与 'nope' 匹配。

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: https://other.example 处没有已注册的服务器。请运行 'pasejo server login https://other.example'。

$ pasejo org devices
? 1
Error: 此设备未登录 https://pasejo.example。请运行 'pasejo server login https://pasejo.example'。

```
