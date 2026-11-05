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
Error: オプション 'ignore-missing-identities' の値 'tru' が無効です。true, false, 1, 0, yes, no, y, n のいずれかを指定してください

$ pasejo store decrypt
? 1
Error: 復号を拒否しました: すべてのシークレットが平文で stdout に出力されます。確認のため --yes-i-know を指定してください。

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Chromium系ブラウザの拡張機能IDは展開された拡張機能の公開鍵から導出されるため、ブラウザ 'chrome' には --extension-id が必要です。

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' は firefox の有効なネイティブメッセージングホスト名ではありません。英字・数字・アンダースコアからなる部分をドットで区切る必要があり、firefox 以外のブラウザーでは小文字でなければなりません。

```
