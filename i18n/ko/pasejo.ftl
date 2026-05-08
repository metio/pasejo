# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## 수신자

recipient-added = '{ $public_key }' 수신자가 추가되었습니다
recipient-removed = '{ $public_key }' 수신자가 제거되었습니다
recipient-does-not-exist-ignored = '{ $public_key }' 수신자가 저장소에 존재하지 않습니다 - 무시합니다

## 비밀

secret-added = '{ $secret_path }'에 비밀이 추가되었습니다
secret-edited = '{ $secret_path }'의 비밀을 편집했습니다
secret-generated = '{ $secret_path }'에 비밀을 생성했습니다
secret-copied = 비밀을 '{ $source_path }'에서 '{ $target_path }'(으)로 복사했습니다
secret-moved = 비밀을 '{ $source_path }'에서 '{ $target_path }'(으)로 이동했습니다
secret-removed = '{ $secret_path }'의 비밀을 제거했습니다
secret-show-as-qrcode = '{ $secret_path }'의 비밀을 QR 코드로 표시합니다
secret-show-as-text = '{ $secret_path }'의 비밀을 텍스트로 표시합니다
secret-copy-into-clipboard = 비밀 '{ $secret_path }'(을)를 클립보드에 복사했습니다 — { $duration } 후에 지워집니다 (지금 지우려면 Ctrl-C)

## 일회용 비밀번호

one-time-password-added = '{ $password_path }'에 일회용 비밀번호가 추가되었습니다
one-time-password-copied = 일회용 비밀번호를 '{ $source_path }'에서 '{ $target_path }'(으)로 복사했습니다
one-time-password-moved = 일회용 비밀번호를 '{ $source_path }'에서 '{ $target_path }'(으)로 이동했습니다
one-time-password-removed = '{ $password_path }'의 일회용 비밀번호가 제거되었습니다
one-time-password-show = '{ $password_path }'의 일회용 비밀번호를 표시합니다
one-time-password-copy-into-clipboard = 일회용 비밀번호 '{ $password_path }'(을)를 클립보드에 복사했습니다 — { $duration } 후에 지워집니다 (지금 지우려면 Ctrl-C)

## 신원

identity-added = '{ $identity_file }' 파일을 사용하는 신원이 추가되었습니다
identity-removed = '{ $identity_file }' 파일을 사용하는 신원이 제거되었습니다
no-identities-exist-yet = '{ $store_name }' 저장소에 아직 신원이 없습니다. 'pasejo identity add ...' 명령으로 추가해 주세요.

## 저장소

store-add-success = '{ $store_path }'에 '{ $store_name }' 저장소가 추가되었습니다
store-set-default = 이제 '{ $store_name }' 저장소가 기본값입니다
store-remove-success = '{ $store_name }' 저장소가 제거되었습니다

## 훅 실행

execute-pull-hooks = '{ $store_name }' 저장소의 pull 훅을 실행하는 중입니다
execute-push-hooks = '{ $store_name }' 저장소의 push 훅을 실행하는 중입니다

## 병합 충돌

merge-conflict-recipient-names = 공개 키 '{ $public_key }'의 수신자에 병합 충돌: 이름 '{ $first_name }'와(과) '{ $second_name }'(이)가 다릅니다
merge-conflict-recipient-removed-and-renamed = 공개 키 '{ $public_key }'의 수신자에 병합 충돌: 한 버전에서 제거되었고, 다른 버전에서 '{ $new_name }'(으)로 이름이 변경되었습니다
merge-conflict-values = '{ $secret_path }'의 { $value_type }에 병합 충돌: 두 버전의 값이 다릅니다
merge-conflict-removed-and-modified = '{ $secret_path }'의 { $value_type }에 병합 충돌: 한 버전에서 { $value_type }(이)가 제거되었고, 다른 버전에서 수정되었습니다

## 목록 출력 (stdout)

list-global-identity = 전역: { $identity_file }
list-store-identity = 저장소: { $identity_file }
list-global-pull-hook = 전역 pull: { $command }
list-global-push-hook = 전역 push: { $command }
list-store-pull-hook = 저장소 pull: { $command }
list-store-push-hook = 저장소 push: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (기본)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## 클립보드

clipboard-read-for-compare-failed = 비교를 위해 클립보드를 읽지 못했습니다: { $error }
clipboard-ctrlc-handler-install-failed = Ctrl-C 핸들러를 설치하지 못했습니다: { $error }. 클립보드는 설정된 시간 초과 후에만 지워집니다.
clipboard-clear-failed = 클립보드를 지우지 못했습니다: { $error }
clipboard-manual-clear-required = 클립보드를 자동으로 지우지 못했습니다 — 지금 수동으로 지워 주세요.
clipboard-notification-dispatch-failed = 클립보드 지우기 알림을 표시하지 못했습니다: { $error }
clipboard-drop-clear-failed = 정리 중 클립보드를 지우지 못했습니다: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] 클립보드를 지웠습니다 (취소됨)
   *[false] 클립보드를 지웠습니다
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] 클립보드를 그대로 두었습니다 (다른 항목을 복사했습니다) (취소됨)
   *[false] 클립보드를 그대로 두었습니다 (다른 항목을 복사했습니다)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] 클립보드를 지웠습니다 (내용을 확인할 수 없습니다) (취소됨)
   *[false] 클립보드를 지웠습니다 (내용을 확인할 수 없습니다)
  }
clipboard-notification-failed = { $cancelled ->
    [true] 클립보드를 지우지 못했습니다! 수동으로 지워 주세요. (취소됨)
   *[false] 클립보드를 지우지 못했습니다! 수동으로 지워 주세요.
  }
