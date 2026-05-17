```
$ pasejo secret add some-secret
? 1
Error: 設定にストアが見つかりません。最初に 'pasejo store add ...' を実行して追加してください

$ pasejo store add --path store --name primary
ストア 'primary' を '[CWD]/store' に追加しました

$ pasejo store add --path other --name primary
? 1
Error: ストア名は既に存在します。別の名前を使用してください。

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': 名前 'missing' のストアは設定に存在しません

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
