```
$ pasejo recipient list
? 1
Error: 저장소 'team'은(는) 원격 저장소이므로 수신자, 훅, 병합이 적용되지 않습니다.

$ pasejo hook get
? 1
Error: 저장소 'team'은(는) 원격 저장소이므로 수신자, 훅, 병합이 적용되지 않습니다.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30'은(는) YYYY-MM-DD 형식의 날짜가 아닙니다.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: 저장소 'files'은(는) 원격 저장소가 아닙니다.

$ pasejo org devices --org nope
? 1
Error: 'nope'과(와) 일치하는 등록된 조직이 없습니다.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: https://other.example에 등록된 서버가 없습니다. 'pasejo server login https://other.example'을 실행하세요.

$ pasejo org devices
? 1
Error: 이 기기는 https://pasejo.example에 로그인되어 있지 않습니다. 'pasejo server login https://pasejo.example'을 실행하세요.

```
