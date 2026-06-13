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
Error: 옵션 'ignore-missing-identities'의 값 'tru'이(가) 잘못되었습니다. true, false, 1, 0, yes, no, y, n 중 하나를 입력하세요

$ pasejo store decrypt
? 1
Error: 복호화를 거부했습니다: 모든 비밀이 평문으로 stdout에 출력됩니다. 확인을 위해 --yes-i-know를 전달하세요.

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Chromium 계열 브라우저의 확장 프로그램 ID는 압축 해제된 확장 프로그램의 공개 키에서 파생되므로 브라우저 'chrome'에는 --extension-id가 필요합니다.

```
