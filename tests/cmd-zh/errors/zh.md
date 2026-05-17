```
$ pasejo secret add some-secret
? 1
Error: 配置中找不到存储。请先运行 'pasejo store add ...' 添加一个

$ pasejo store add --path store --name primary
已在 '[CWD]/store' 添加存储 'primary'

$ pasejo store add --path other --name primary
? 1
Error: 存储名称已存在。请使用其他名称。

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': 名为 'missing' 的存储在配置中不存在

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
