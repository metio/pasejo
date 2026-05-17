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

## 프롬프트

prompt-enter-secret = { $secret_path }의 비밀을 입력하세요:
prompt-could-not-read-secret = { $secret_path }의 비밀을 읽을 수 없습니다
prompt-overwrite-secret = 기존 비밀을 덮어쓰시겠습니까?
prompt-remove-secret = 기존 비밀을 제거하시겠습니까?
prompt-overwrite-one-time-password = 기존 일회용 비밀번호를 덮어쓰시겠습니까?
prompt-remove-one-time-password = 기존 일회용 비밀번호를 제거하시겠습니까?

## 오류 메시지

error-cannot-get-user-confirmation = 사용자 확인을 가져올 수 없습니다
error-no-confirmation-from-non-terminal = 터미널이 아닌 입력에서는 사용자 확인을 가져올 수 없습니다. 확인을 건너뛰려면 --force를 사용하세요.
error-could-not-load-configuration = 구성을 불러올 수 없습니다
error-store-does-not-exist = 이름 '{ $store_name }'을(를) 가진 저장소가 구성에 존재하지 않습니다
error-invalid-line-number = '{ $input }'은(는) 유효한 줄 번호가 아닙니다
error-line-number-must-not-be-zero = 줄 번호는 0이 될 수 없습니다. 첫 번째 줄은 1, 마지막 줄은 -1을 사용하세요
error-invalid-count = '{ $input }'은(는) 유효한 개수가 아닙니다
error-count-must-not-be-zero = 개수는 0이 될 수 없습니다. 첫 번째 줄을 건너뛰려면 1을 사용하세요
error-file-does-not-exist = 파일 '{ $input }'이(가) 존재하지 않습니다
error-secret-already-exists = 비밀이 이미 { $secret_path }에 존재합니다. 덮어쓰려면 --force를 사용하세요.
error-secret-already-exists-overwrite-or-inplace = 비밀이 이미 { $secret_path }에 존재합니다. 완전히 덮어쓰려면 --force, 첫 번째 줄만 수정하려면 --inplace를 사용하세요.
error-no-secret-found = '{ $secret_path }'에서 비밀을 찾을 수 없습니다
error-not-allowed-to-remove-secret = { $secret_path }의 비밀 제거가 허용되지 않습니다. 덮어쓰려면 --force를 사용하세요.
error-secret-does-not-exist-for-edit = { $secret_path }에 비밀이 존재하지 않습니다. 생성하려면 'pasejo secret add'를 사용하세요.
error-one-time-password-already-exists = 일회용 비밀번호가 이미 { $password_path }에 존재합니다. 덮어쓰려면 --force를 사용하세요.
error-not-allowed-to-remove-one-time-password = { $password_path }의 일회용 비밀번호 제거가 허용되지 않습니다. 덮어쓰려면 --force를 사용하세요.
error-no-one-time-password-found = '{ $password_path }'에서 일회용 비밀번호를 찾을 수 없습니다
error-no-store-in-configuration = 구성에서 저장소를 찾을 수 없습니다. 먼저 'pasejo store add ...'를 실행하여 추가하세요
error-no-store-or-global = 구성에서 저장소를 찾을 수 없고 --global 플래그도 지정되지 않았습니다. 먼저 'pasejo store add ...'를 실행하여 추가하세요
error-store-name-already-exists = 저장소 이름이 이미 존재합니다. 다른 이름을 사용하세요.
error-store-path-is-directory = 디렉터리를 저장소 경로로 사용할 수 없습니다. 파일 경로를 사용하세요.
error-cannot-create-store-path = 저장소 경로를 생성할 수 없습니다. 경로를 확인하고 다시 시도하세요.
error-cannot-get-store-parent = 저장소 경로의 상위 디렉터리를 가져올 수 없습니다. 경로를 확인하고 다시 시도하세요.
error-cannot-identify-store = 저장소를 식별할 수 없습니다. 기본 저장소를 설정하거나 --store로 저장소를 지정하거나 --global로 전역 ID를 설정하세요.
error-no-identity-files-to-decrypt = 복호화할 ID 파일이 없습니다. 저장소 초기화를 완료하려면 ID를 하나 이상 추가하세요.
error-invalid-ssh-public-key-format = 유효하지 않은 SSH 공개 키 형식
error-username-empty = 사용자 이름은 비어 있을 수 없습니다
error-username-contains-dotdot = 유효하지 않은 사용자 이름 '{ $username }': '..'을(를) 포함할 수 없습니다
error-username-invalid-character = 유효하지 않은 사용자 이름 '{ $username }': ASCII 문자 또는 숫자로 시작해야 하며 ASCII 문자, 숫자, '-', '_', '.'만 포함할 수 있습니다
error-no-public-key-found-in-file = '{ $filename }'에서 공개 키를 찾을 수 없습니다
error-no-public-key-source = 공개 키의 소스를 하나 이상 지정해야 합니다
error-no-qrcode-found = '{ $qrcode }'에서 QR 코드를 찾을 수 없습니다
error-failed-to-decode-qrcode = '{ $qrcode }'의 QR 코드 디코딩 실패
error-cannot-determine-store-name = 저장소 이름을 확인할 수 없습니다
error-cannot-parse-hook-command = 명령을 파싱할 수 없습니다: { $command }
error-empty-hook-command = 빈 훅 명령: { $command }
error-failed-to-run-hook = 훅 { $command } 실행 실패
error-hook-failed-no-detail = 훅 { $command } 실패 (종료 { $exit })
error-hook-failed-with-detail = 훅 { $command } 실패 (종료 { $exit }): { $detail }
error-cannot-determine-store-parent-path = 저장소 경로 { $path }의 상위를 확인할 수 없습니다
error-store-path-not-utf8 = 토큰 { $token }에서 %p를 대체할 수 없습니다: 저장소 경로 { $path }은(는) 유효한 UTF-8이 아닙니다
error-merge-conflict-recipients = 수신자에서 병합 충돌이 감지되었습니다. 수동으로 해결하세요.
error-recipient-not-found-in-store = 수신자를 저장소에서 찾을 수 없습니다
error-cannot-decrypt-common-ancestor-store = 공통 조상 저장소를 복호화할 수 없습니다
error-cannot-decrypt-current-version-store = 현재 버전 저장소를 복호화할 수 없습니다
error-cannot-decrypt-other-version-store = 다른 버전 저장소를 복호화할 수 없습니다
error-cannot-read-file = '{ $path }'의 파일을 읽을 수 없습니다
error-downloading-public-key-failed = { $provider }에서 공개 키 다운로드 실패
error-cannot-determine-parent-directory = { $path }의 상위 디렉터리를 확인할 수 없습니다
error-failed-to-create-directory = 디렉터리 { $path } 생성 실패
error-failed-to-write-file = { $path } 쓰기 실패
error-failed-to-fsync-file = { $path } fsync 실패
error-failed-to-rename-file = { $from }을(를) { $to }(으)로 이름 바꾸기 실패
error-failed-to-create-file = { $path } 생성 실패
error-could-not-resolve-config-path = 구성의 절대 경로를 확인할 수 없었습니다
error-could-not-determine-config-path = 구성 경로를 확인할 수 없었습니다
error-could-not-resolve-store-path = 저장소의 절대 경로를 확인할 수 없었습니다
error-could-not-create-config-dir = 구성 디렉터리를 생성할 수 없었습니다
error-could-not-migrate-legacy-config = 레거시 구성 파일을 마이그레이션할 수 없었습니다
error-could-not-move-file = 파일을 이동할 수 없었습니다
error-could-not-copy-file = 파일을 복사할 수 없었습니다
error-could-not-remove-source-after-copy = 복사 후 원본 파일을 제거할 수 없었습니다
error-could-not-read-configuration = 구성을 읽을 수 없었습니다
error-could-not-serialize-migrated-config = 마이그레이션된 구성을 직렬화할 수 없었습니다
error-could-not-store-configuration = 구성을 저장할 수 없었습니다
error-could-not-load-migrated-config = 마이그레이션된 구성을 불러올 수 없었습니다
error-could-not-serialize-configuration = 구성을 직렬화할 수 없었습니다
error-config-not-valid-toml = { $path }의 구성 파일이 유효한 TOML이 아닙니다
error-could-not-open-store-for-lock = 잠금을 위해 저장소 파일을 열 수 없었습니다: { $path }
error-could-not-acquire-store-lock = 저장소 파일의 잠금을 획득할 수 없었습니다: { $path }
error-cannot-encrypt-store = 저장소를 암호화할 수 없습니다
error-failed-to-run-command = 명령 { $binary } 실행 실패
error-command-exited-with = 명령 { $binary }이(가) { $exit }(으)로 종료되었습니다
