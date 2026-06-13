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
Error: 选项 'ignore-missing-identities' 的值 'tru' 无效：应为 true、false、1、0、yes、no、y、n 之一

$ pasejo store decrypt
? 1
Error: 拒绝解密:这会将每个密钥以明文形式打印到 stdout。请传递 --yes-i-know 以确认。

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Chromium 系浏览器的扩展 ID 由解压扩展的公钥派生，因此浏览器 'chrome' 需要 --extension-id。

```
