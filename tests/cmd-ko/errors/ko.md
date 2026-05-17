```
$ pasejo secret add some-secret
? 1
Error: 구성에서 저장소를 찾을 수 없습니다. 먼저 'pasejo store add ...'를 실행하여 추가하세요

$ pasejo store add --path store --name primary
'[CWD]/store'에 'primary' 저장소가 추가되었습니다

$ pasejo store add --path other --name primary
? 1
Error: 저장소 이름이 이미 존재합니다. 다른 이름을 사용하세요.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': 이름 'missing'을(를) 가진 저장소가 구성에 존재하지 않습니다

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
