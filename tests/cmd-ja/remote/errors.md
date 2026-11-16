```
$ pasejo recipient list
? 1
Error: ストア 'team' はリモートストアです。受信者、フック、マージは適用されません。

$ pasejo hook get
? 1
Error: ストア 'team' はリモートストアです。受信者、フック、マージは適用されません。

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' は YYYY-MM-DD 形式の日付ではありません。

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: ストア 'files' はリモートストアではありません。

$ pasejo org devices --org nope
? 1
Error: 'nope' に一致する登録済みの組織はありません。

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: https://other.example にはサーバーが登録されていません。'pasejo server login https://other.example' を実行してください。

$ pasejo org devices
? 1
Error: このデバイスは https://pasejo.example にサインインしていません。'pasejo server login https://pasejo.example' を実行してください。

```
