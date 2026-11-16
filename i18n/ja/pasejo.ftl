# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## 受信者

recipient-added = 受信者 '{ $public_key }' を追加しました
recipient-removed = 受信者 '{ $public_key }' を削除しました
recipient-does-not-exist-ignored = 受信者 '{ $public_key }' はストアに存在しません - 無視します
recipient-unsupported-key-skipped = { $provider } ユーザー '{ $username }' の種類 '{ $key_type }' の鍵をスキップします: age はこの鍵に暗号化できません

## シークレット

secret-added = '{ $secret_path }' にシークレットを追加しました
secret-edited = '{ $secret_path }' のシークレットを編集しました
secret-generated = '{ $secret_path }' にシークレットを生成しました
secret-copied = シークレットを '{ $source_path }' から '{ $target_path }' にコピーしました
secret-moved = シークレットを '{ $source_path }' から '{ $target_path }' に移動しました
secret-removed = '{ $secret_path }' のシークレットを削除しました
secret-show-as-qrcode = '{ $secret_path }' のシークレットをQRコードで表示します
secret-show-as-text = '{ $secret_path }' のシークレットをテキストで表示します
secret-copy-into-clipboard = シークレット '{ $secret_path }' をクリップボードにコピーしました — { $duration } 後に消去されます (今すぐ消去するには Ctrl-C)

## ワンタイムパスワード

one-time-password-added = '{ $password_path }' にワンタイムパスワードを追加しました
one-time-password-copied = ワンタイムパスワードを '{ $source_path }' から '{ $target_path }' にコピーしました
one-time-password-moved = ワンタイムパスワードを '{ $source_path }' から '{ $target_path }' に移動しました
one-time-password-removed = '{ $password_path }' のワンタイムパスワードを削除しました
one-time-password-show = '{ $password_path }' のワンタイムパスワードを表示します
one-time-password-copy-into-clipboard = ワンタイムパスワード '{ $password_path }' をクリップボードにコピーしました — { $duration } 後に消去されます (今すぐ消去するには Ctrl-C)

## アイデンティティ

identity-added = ファイル '{ $identity_file }' を使用するアイデンティティを追加しました
identity-removed = ファイル '{ $identity_file }' を使用するアイデンティティを削除しました
no-identities-exist-yet = ストア '{ $store_name }' にはまだアイデンティティがありません。'pasejo identity add ...' で追加してください。

## ストア

store-add-success = ストア '{ $store_name }' を '{ $store_path }' に追加しました
store-set-default = ストア '{ $store_name }' をデフォルトに設定しました
store-remove-success = ストア '{ $store_name }' を削除しました
store-remove-success-with-data = ストア '{ $store_name }' を削除し、暗号化されたデータファイルも削除しました

## フックの実行

execute-pull-hooks = ストア '{ $store_name }' のプルフックを実行しています
execute-push-hooks = ストア '{ $store_name }' のプッシュフックを実行しています
hook-run-nothing-selected = 実行するものがありません。--pull や --push を指定して実行するフックを選択してください。
export-plaintext-warning = 警告: エクスポートにはすべてのシークレットが平文で含まれます。
export-hotp-omitted = 警告: Bitwarden は HOTP ワンタイムパスワードを保存できないため、次のものはエクスポートに含まれません: { $names }
export-orphaned-otp-omitted = 警告: 次のワンタイムパスワードには同名のシークレットがないため、エクスポートに含まれません: { $names }
export-written-to-file = エクスポートを '{ $path }' に書き込みました

## マージ競合

merge-conflict-recipient-names = 公開鍵 '{ $public_key }' の受信者でマージ競合: 名前 '{ $first_name }' と '{ $second_name }' が異なります
merge-conflict-recipient-removed-and-renamed = 公開鍵 '{ $public_key }' の受信者でマージ競合: 一方のバージョンでは削除され、もう一方では '{ $new_name }' に名前変更されました
merge-conflict-values = '{ $secret_path }' の { $value_type } でマージ競合: 二つのバージョンで値が異なります
merge-conflict-removed-and-modified = '{ $secret_path }' の { $value_type } でマージ競合: 一方のバージョンでは削除され、もう一方では変更されました

## リスト出力 (stdout)

list-global-identity = グローバル: { $identity_file }
list-store-identity = ストア: { $identity_file }
list-store-identity-also-global = ストア: { $identity_file } (グローバルにも登録済み)
list-global-pull-hook = グローバルプル: { $command }
list-global-push-hook = グローバルプッシュ: { $command }
list-store-pull-hook = ストアプル: { $command }
list-store-push-hook = ストアプッシュ: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (デフォルト)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## クリップボード

clipboard-read-for-compare-failed = 比較のためのクリップボード読み取りに失敗: { $error }
clipboard-ctrlc-handler-install-failed = Ctrl-C ハンドラのインストールに失敗: { $error }。クリップボードは設定されたタイムアウト後にのみクリアされます。
clipboard-clear-failed = クリップボードのクリアに失敗: { $error }
clipboard-manual-clear-required = クリップボードを自動的にクリアできませんでした — 今すぐ手動でクリアしてください。
clipboard-notification-dispatch-failed = クリップボードクリア通知の表示に失敗: { $error }
clipboard-drop-clear-failed = クリーンアップ中のクリップボードのクリアに失敗: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] クリップボードをクリアしました (キャンセル)
   *[false] クリップボードをクリアしました
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] クリップボードはそのまま (他の内容がコピーされました) (キャンセル)
   *[false] クリップボードはそのまま (他の内容がコピーされました)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] クリップボードをクリア (内容を確認できませんでした) (キャンセル)
   *[false] クリップボードをクリア (内容を確認できませんでした)
  }
clipboard-notification-failed = { $cancelled ->
    [true] クリップボードのクリアに失敗しました！手動でクリアしてください。(キャンセル)
   *[false] クリップボードのクリアに失敗しました！手動でクリアしてください。
  }

## プロンプト

prompt-enter-secret = { $secret_path } のシークレットを入力:
prompt-could-not-read-secret = { $secret_path } のシークレットを読み込めませんでした
error-editor-launch-failed = エディター '{ $editor }' を起動できませんでした
error-editor-exited-unsuccessfully = エディター '{ $editor }' が正常に終了しなかったため、シークレットは変更されていません
error-editor-file-not-utf8 = 編集されたシークレットは有効な UTF-8 ではありません
error-editor-file-create-failed = '{ $path }' にエディター用の一時ファイルを作成できませんでした
editor-file-in-temp-dir = $XDG_RUNTIME_DIR も /dev/shm も利用できないため、エディターの実行中はシークレットが '{ $path }' 内のファイルに書き込まれます。このディレクトリはディスク上にある可能性があります。
editor-file-cleanup-failed = エディターの一時ファイル '{ $path }' を上書きして削除できませんでした: { $error }
prompt-overwrite-secret = 既存のシークレットを上書きしますか？
prompt-remove-secret = 既存のシークレットを削除しますか？
prompt-overwrite-one-time-password = 既存のワンタイムパスワードを上書きしますか？
prompt-remove-one-time-password = 既存のワンタイムパスワードを削除しますか？

## エラーメッセージ

error-cannot-get-user-confirmation = ユーザーの確認を取得できません
error-no-confirmation-from-non-terminal = 端末以外の入力からはユーザー確認を取得できません。--force を使用して確認をスキップしてください。
error-could-not-load-configuration = 設定を読み込めませんでした
error-store-does-not-exist = 名前 '{ $store_name }' のストアは設定に存在しません
error-invalid-line-number = '{ $input }' は有効な行番号ではありません
error-line-number-must-not-be-zero = 行番号は 0 にできません。最初の行には 1、最後の行には -1 を使用してください
error-invalid-count = '{ $input }' は有効な件数ではありません
error-count-must-not-be-zero = 件数は 0 にできません。最初の行をスキップするには 1 を使用してください
error-file-does-not-exist = ファイル '{ $input }' は存在しません
error-secret-already-exists = シークレットは既に { $secret_path } に存在します。上書きするには --force を使用してください。
error-secret-already-exists-overwrite-or-inplace = シークレットは既に { $secret_path } に存在します。完全に上書きするには --force、最初の行のみを変更するには --inplace を使用してください。
error-password-length-zero = パスワードの長さを 0 にすることはできません
error-password-length-too-short = パスワードの長さが必要な文字の種類に対して短すぎます
error-password-no-character-kinds = 少なくとも 1 種類の文字を有効にする必要があります
error-password-generation-failed = パスワードを生成できません: { $reason }
error-no-secret-found = '{ $secret_path }' にシークレットが見つかりません
error-not-allowed-to-remove-secret = { $secret_path } のシークレットの削除は許可されていません。上書きするには --force を使用してください。
error-secret-does-not-exist-for-edit = { $secret_path } にシークレットが存在しません。作成するには 'pasejo secret add' を使用してください。
error-one-time-password-already-exists = ワンタイムパスワードは既に { $password_path } に存在します。上書きするには --force を使用してください。
error-not-allowed-to-remove-one-time-password = { $password_path } のワンタイムパスワードの削除は許可されていません。上書きするには --force を使用してください。
error-no-one-time-password-found = '{ $password_path }' にワンタイムパスワードが見つかりません
error-invalid-one-time-password-secret = ワンタイムパスワードのシークレットは空でない base32 文字列 (英字 A-Z と数字 2-7) である必要があります
error-invalid-otpauth-url = otpauth URL が有効なワンタイムパスワードを表していません
error-hotp-counter-exhausted = HOTP カウンターが最大値に達したため、これ以上進められません
error-no-store-in-configuration = 設定にストアが見つかりません。最初に 'pasejo store add ...' を実行して追加してください
error-unknown-store = ストア '{ $store_name }' は設定に存在しません。'pasejo store list' を実行して設定済みのストアを確認してください。
error-decrypt-requires-yes-i-know = 復号を拒否しました: すべてのシークレットが平文で stdout に出力されます。確認のため --yes-i-know を指定してください。
error-no-store-or-global = 設定にストアが見つからず、--global フラグも指定されていません。最初に 'pasejo store add ...' を実行して追加してください
error-store-name-already-exists = ストア名は既に存在します。別の名前を使用してください。
error-store-path-is-directory = ディレクトリをストアパスとして使用できません。ファイルパスを使用してください。
error-cannot-create-store-path = ストアパスを作成できません。パスを確認してやり直してください。
error-cannot-get-store-parent = ストアパスの親ディレクトリを取得できません。パスを確認してやり直してください。
error-cannot-identify-store = ストアを識別できません。デフォルトストアを設定するか、--store でストアを指定するか、--global でアイデンティティをグローバルに設定してください。
error-identity-file-already-added = '--file <FILE>' の値 '{ $file }' は無効です: ファイルは既にアイデンティティとして追加されています
error-identity-file-not-known = '--file <FILE>' の値 '{ $file }' は無効です: ファイルは既知のアイデンティティと一致しません
error-identity-did-you-mean = '{ $stored }' のことですか？アイデンティティのパスは完全一致で比較されます。
error-no-identity-files-to-decrypt = 復号するアイデンティティファイルがありません。ストアの初期化を完了するには、少なくとも 1 つのアイデンティティを追加してください。
error-invalid-ssh-public-key-format = 無効な SSH 公開鍵形式
error-username-empty = ユーザー名は空であってはなりません
error-username-contains-dotdot = 無効なユーザー名 '{ $username }': '..' を含めてはなりません
error-username-invalid-character = 無効なユーザー名 '{ $username }': ASCII の英字または数字で始まり、ASCII の英字、数字、'-'、'_'、'.' のみを含む必要があります
error-no-public-key-found-in-file = '{ $filename }' に公開鍵が見つかりません
error-no-public-keys-for-user = { $provider } ユーザー '{ $username }' には受信者として追加できる公開 SSH 鍵がありません
error-no-public-key-source = 公開鍵のソースを少なくとも 1 つ指定する必要があります
error-no-qrcode-found = '{ $qrcode }' に QR コードが見つかりません
error-failed-to-decode-qrcode = '{ $qrcode }' の QR コードのデコードに失敗しました
error-qr-sandbox-failed = QR コードデコーダーをサンドボックス内で実行できませんでした
error-qr-sandbox-child-signal = QR コードデコーダーがシグナル { $signal } によって終了しました
error-qr-sandbox-not-enforced = QR コードデコーダーのサンドボックスを適用できませんでした。実行中のカーネルが Landlock をサポートしていません
error-cannot-determine-store-name = ストア名を特定できません
error-cannot-parse-hook-command = コマンドを解析できません: { $command }
error-empty-hook-command = 空のフックコマンド: { $command }
error-failed-to-run-hook = フック { $command } の実行に失敗しました
error-hook-failed-no-detail = フック { $command } が失敗しました (終了 { $exit })
error-hook-failed-with-detail = フック { $command } が失敗しました (終了 { $exit }): { $detail }
error-cannot-determine-store-parent-path = ストアパス { $path } の親を特定できません
error-store-path-not-utf8 = トークン { $token } の %p を置換できません: ストアパス { $path } は有効な UTF-8 ではありません
error-merge-conflict-recipients = 受信者でマージ競合が検出されました。手動で解決してください。
error-merge-conflict = { $value_type }s でマージ競合が検出されました。手動で解決してください。
error-recipient-not-found-in-store = 受信者がストアに見つかりません
error-cannot-remove-last-recipient = ストアから最後の受信者を削除できません。これを削除する前に新しい受信者を追加してください。
error-cannot-decrypt-common-ancestor-store = 共通祖先ストアを復号できません
error-cannot-decrypt-current-version-store = 現在のバージョンのストアを復号できません
error-cannot-decrypt-other-version-store = 他のバージョンのストアを復号できません
error-cannot-read-file = '{ $path }' のファイルを読み取れません
error-downloading-public-key-failed = { $provider } からの公開鍵のダウンロードに失敗しました
error-download-user-not-found = { $provider } にユーザー '{ $username }' は存在しません
error-download-rate-limited = { $provider } がリクエストを制限しました。少し待ってから再試行してください
error-download-server-error = { $provider } がエラーを返しました (HTTP { $status })
error-download-timeout = { $provider } からの公開鍵のダウンロードがタイムアウトしました。接続を確認するか key-download-timeout-seconds を増やしてください
error-cannot-determine-parent-directory = { $path } の親ディレクトリを特定できません
error-failed-to-create-directory = ディレクトリ { $path } の作成に失敗しました
error-failed-to-write-file = { $path } への書き込みに失敗しました
error-failed-to-fsync-file = { $path } の fsync に失敗しました
error-failed-to-rename-file = { $from } を { $to } へリネームできませんでした
error-failed-to-create-file = { $path } の作成に失敗しました
error-could-not-resolve-config-path = 設定の絶対パスを解決できませんでした
error-could-not-determine-config-path = 設定パスを特定できませんでした
error-could-not-resolve-store-path = ストアの絶対パスを解決できませんでした
error-could-not-create-config-dir = 設定ディレクトリを作成できませんでした
error-could-not-migrate-legacy-config = レガシー設定ファイルを移行できませんでした
error-could-not-move-file = ファイルを移動できませんでした
error-could-not-copy-file = ファイルをコピーできませんでした
error-could-not-remove-source-after-copy = コピー後にソースファイルを削除できませんでした
error-could-not-read-configuration = 設定を読み取れませんでした
error-could-not-serialize-migrated-config = 移行された設定をシリアライズできませんでした
error-could-not-store-configuration = 設定を保存できませんでした
error-could-not-load-migrated-config = 移行された設定を読み込めませんでした
error-could-not-serialize-configuration = 設定をシリアライズできませんでした
error-config-not-valid-toml = { $path } の設定ファイルは有効な TOML ではありません
error-config-invalid-integer = オプション '{ $option }' の値 '{ $value }' が無効です。非負の整数を指定してください
error-config-invalid-boolean = オプション '{ $option }' の値 '{ $value }' が無効です。true, false, 1, 0, yes, no, y, n のいずれかを指定してください
error-could-not-open-store-for-lock = ロック用にストアファイルを開けませんでした: { $path }
error-could-not-acquire-store-lock = ストアファイルのロックを取得できませんでした: { $path }
error-store-lock-wait-timed-out = ストアファイルのロック待機がタイムアウトしました: { $path }
error-cannot-encrypt-store = ストアを暗号化できません
error-failed-to-run-command = コマンド { $binary } の実行に失敗しました
error-command-exited-with = コマンド { $binary } は { $exit } で終了しました
error-no-field-found = シークレット '{ $secret_path }' にフィールド '{ $field }' がありません
error-no-line-found = シークレット '{ $secret_path }' に { $line } 行目がありません
error-value-unresolved = '{ $name }' の値を解決できません
error-env-assignment-invalid = '{ $input }' は VARIABLE=SECRET の形式ではありません
error-env-field-assignment-invalid = '{ $input }' は VARIABLE=SECRET=FIELD の形式ではありません
error-env-variable-name-invalid = '{ $variable }' は有効な環境変数名ではありません。英字、数字、アンダースコアを使用し、数字で始めないでください
error-env-variable-duplicate = 環境変数 { $variable } が複数回割り当てられています
error-env-reference-line-and-field = { $variable } の参照で 'line' と 'field' の両方が設定されています
error-resolve-input-invalid = 標準入力からシークレットの参照を読み取れません
exec-value-too-short-to-redact = { $variable } の値は { $length } 文字未満のため、伏せ字にされません
error-browser-host-requires-pipe = ブラウザホストには、接続中のブラウザ拡張機能からのパイプ標準入力が必要です。このコマンドはブラウザのネイティブメッセージングマニフェスト経由で実行してください。ターミナルから直接実行しないでください。
error-browser-store-file-missing = ストアファイルが存在しません: { $path }
error-browser-identity-passphrase-protected = ブラウザーホストは暗号化されたアイデンティティのパスフレーズを尋ねることができません。このストアには暗号化されていないアイデンティティかプラグインベースのアイデンティティを使用してください。
prompt-browser-host-pinentry = PIN またはパスフレーズ
error-browser-message-too-large = { $length } バイトのメッセージがネイティブメッセージングの上限 { $limit } バイトを超えています
error-browser-response-too-large = { $length } バイトの応答がネイティブメッセージングの上限 { $limit } バイトを超えています
error-browser-install-manifest-no-browser = ネイティブメッセージングマニフェストをインストールする --browser を少なくとも1つ指定してください。
error-browser-install-manifest-needs-extension-id = Chromium系ブラウザの拡張機能IDは展開された拡張機能の公開鍵から導出されるため、ブラウザ '{ $browser }' には --extension-id が必要です。
error-browser-install-manifest-extension-id-multiple-browsers = 各ブラウザには固有の拡張機能 ID が必要なため、--extension-id を複数の --browser と組み合わせることはできません。ブラウザごとにコマンドを実行してください。
error-browser-install-manifest-unsupported-os = ネイティブメッセージングマニフェストのインストールは '{ $os }' ではサポートされていません。
error-browser-install-manifest-registry-failed = ネイティブメッセージングのレジストリキー '{ $key }' を書き込めませんでした
error-browser-install-manifest-no-home-dir = ユーザーのホームディレクトリを特定できませんでした。
error-browser-install-manifest-no-data-dir = ユーザーのデータディレクトリを特定できませんでした。
error-browser-install-manifest-no-current-exe = 実行中の pasejo バイナリのパスを特定できませんでした。
browser-install-manifest-written = '{ $browser }' のネイティブメッセージングマニフェストを '{ $path }' に書き込みました
browser-install-manifest-print-wrapper-note = 注意: 出力されたマニフェストは '{ $path }' のラッパースクリプトを参照していますが、このコマンドはそれをインストールしていません。既存の実行ファイルを参照するには --binary を使用してください。
error-browser-install-manifest-invalid-env = --env エントリ '{ $entry }' は KEY=VALUE 形式ではありません。
error-browser-install-manifest-invalid-env-name = --env 名 '{ $name }' は有効な POSIX 環境変数名ではありません（先頭は英字またはアンダースコアで、英字・数字・アンダースコアのみ使用できます）。
error-browser-install-manifest-invalid-env-value-windows = '{ $name }' の --env の値は Windows ラッパーに書き込めません。二重引用符または改行が含まれています。
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' は { $browser } の有効なネイティブメッセージングホスト名ではありません。英字・数字・アンダースコアからなる部分をドットで区切る必要があり、firefox 以外のブラウザーでは小文字でなければなりません。
error-browser-install-manifest-invalid-extension-id = --extension-id '{ $id }' は有効な拡張機能IDではありません。使用できるのは ASCII 英字、数字、'@'、'.'、'_'、'-'、波括弧のみです。
error-browser-release-log-lock = ブラウザー提供ログをロックできませんでした: { $path }
error-browser-release-log-no-data-dir = ブラウザー提供ログのデータディレクトリを特定できませんでした。
error-browser-release-log-failed = ブラウザー提供ログに記録できなかったため、認証情報は送信されませんでした。
error-browser-caller-not-allowed = ブラウザ拡張機能 '{ $caller }' はこの pasejo ホストを使用できません。この拡張機能に対して `pasejo browser install-manifest` を実行するか、--allowed-caller で追加してください。
error-browser-caller-unnamed = ブラウザがこの pasejo ホストを起動した拡張機能を示さなかったため、--allowed-caller と照合できません。
browser-log-empty = ブラウザーに提供された認証情報はまだありません。
browser-log-unreadable-lines = ブラウザー提供ログの読み取れない { $count } 行をスキップしました。
browser-log-header-time = 時刻
browser-log-header-domain = ドメイン
browser-log-header-store = ストア
browser-log-header-secret = シークレット
browser-log-header-caller = 呼び出し元

## Remote stores
error-invalid-date = '{ $value }' は YYYY-MM-DD 形式の日付ではありません。
error-remote-actions-failed = 保留中のアクションの一部が失敗しました。
error-remote-bad-request = サーバーはリクエストを受け付けませんでした。
error-remote-checkpoint-key-changed = { $url } のチェックポイント鍵が初回サインイン時に固定したものと異なります。サーバーが置き換えられた可能性があります。再度サインインする前に運用者に確認してください。
error-remote-concurrent-change = 他の誰かが同時に '{ $name }' を変更しました。新しいバージョンに変更を適用するにはコマンドを再実行してください。
error-remote-conflict = サーバーにより新しいバージョンがあります。コマンドを再実行してください。
error-remote-device-user-not-member = { $email } は組織のメンバーではありません。先に 'pasejo org role { $email } member' で追加してください。
error-remote-fingerprint-mismatch = フィンガープリントが一致しません。サーバーの値は { $expected } です。
error-remote-fingerprint-not-confirmed = フィンガープリントが確認されませんでした。
error-remote-fingerprint-required = フィンガープリント { $expected } を別の経路で照合してから --fingerprint で指定してください。
error-remote-forbidden = サーバーが拒否しました。この操作の権限がありません。
error-remote-invalid-id = '{ $value }' は有効な識別子ではありません。
error-remote-invalid-recipient = '{ $recipient }' はサーバーが暗号化できる age 受信者ではありません。
error-remote-login-denied = ブラウザーでサインインが拒否されました。
error-remote-login-expired = サインインが時間内に承認されませんでした。'pasejo server login' を再実行してください。
error-remote-needs-second-admin = この変更には 2 人目の管理者の署名が必要です。
error-remote-no-data-dir = このデバイスの鍵を保存するローカルデータディレクトリがありません。PASEJO_DATA_DIR を設定してください。
error-remote-no-org = 組織が登録されていません。'pasejo org create' または 'pasejo org join' を実行してください。
error-remote-no-recovery-recipient = 組織 { $org } には復旧用の受信者がありません。
error-remote-no-server = サーバーが登録されていません。'pasejo server login URL' を実行してください。
error-remote-no-session = このデバイスはサインインしていません。
error-remote-not-found = サーバーは要求されたものを認識していません。
error-remote-not-logged-in = このデバイスは { $url } にサインインしていません。'pasejo server login { $url }' を実行してください。
error-remote-org-ambiguous = '{ $org }' に一致する組織が複数あります。組織の識別子を指定してください。
error-remote-org-required = 複数の組織が登録されています。--org で 1 つを選んでください。
error-remote-pending-device = このデバイスは組織の管理者または承認者による承認を待っています。
error-remote-rate-limited = サーバーは再試行の前に待つよう求めています。
error-remote-recipient-required = このデバイス用の age 受信者を提供するグローバル ID がありません。--recipient で指定してください。
error-remote-rejected = 組織のメンバーシップログが変更を拒否しました。
error-remote-server-ambiguous = 複数のサーバーが登録されています。サーバーの URL を指定してください。
error-remote-server-failed = サーバーはリクエストに応答できませんでした。
error-remote-session-rejected = サーバーはこのデバイスのセッションを受け付けなくなりました。'pasejo server login' を再実行してください。
error-remote-signing-key-unreadable = { $path } にあるデバイスの署名鍵を読み取れません。
error-remote-store-command-not-applicable = ストア '{ $store_name }' はリモートストアです。受信者、フック、マージは適用されません。
error-remote-store-name-taken = '{ $name }' という名前のストアは既に存在します。
error-remote-store-required = ストア '{ $store_name }' はリモートストアではありません。
error-remote-team-exists = '{ $name }' という名前のチームは既に存在します。
error-remote-too-large = リクエストがサーバーの受け付ける上限を超えています。
error-remote-unavailable = このデバイスにはリクエストに必要なものがありません。
error-remote-unknown-device = そのようなデバイスはありません: { $device }。
error-remote-unknown-org = '{ $org }' に一致する登録済みの組織はありません。
error-remote-unknown-proposal = 識別子 { $proposal } の未処理の提案はありません。
error-remote-unknown-server = { $url } にはサーバーが登録されていません。'pasejo server login { $url }' を実行してください。
error-remote-unknown-store = 組織にストア { $store } はありません。
error-remote-unknown-team = 組織に '{ $name }' という名前のチームはありません。
error-remote-unknown-user = 組織内にメールアドレス { $email } を使う人はいません。
error-remote-unreachable = サーバーに接続できません。
error-remote-verification-failed = サーバーが返した内容を検証できません。運用者が理由を説明するまで、このサーバーを信頼しないでください。
error-remote-workload-token-unreadable = { $path } にあるワークロードトークンを読み取れません。
error-remote-wrong-object-kind = エントリ '{ $name }' に想定外の種類のデータが含まれています。
prompt-remote-confirm-fingerprint = フィンガープリント { $fingerprint } は相手側に表示されたものと一致しますか？
remote-action-approve-device = { $email } のデバイス '{ $label }' を承認 (フィンガープリント { $fingerprint })
remote-action-cosign-checkpoint = 監査チェックポイント { $seq } に連署
remote-action-join-member = { $email } をメンバーとして追加
remote-action-remove-member = { $user } を組織から削除
remote-action-revoke-grant = ストア { $store } に対する { $principal } の権限を取り消し
remote-action-rotate-epoch = ストア { $store } の鍵をローテーション
remote-action-sync-team = チーム { $team } のメンバーを更新
remote-action-wrap-keys = { $count } 台のデバイスにストア { $store } の鍵を渡す
remote-device-state-active = 有効
remote-device-state-pending = 保留中
remote-action-done = 完了: { $action }
remote-action-skipped = スキップ: { $action } ({ $reason })
remote-action-failed = 失敗: { $action }: { $error }
remote-action-pending = 保留中: { $action }
remote-audit-line = { $seq }  { $time }  { $actor }  { $action }  { $outcome }  { $store }  { $detail }
remote-device-line = { $id }  { $label }  { $email }  { $fingerprint }  { $state }
remote-proposal-line = { $proposal }  { $kind }  { $author }
remote-store-line = { $store }  { $label }  { $role }  { $local }
remote-rotation-item = { $name }
remote-status-server = { $url }: デバイス { $device } ({ $label })、フィンガープリント { $fingerprint }、{ $session ->
    [yes] サインイン済み
   *[other] 未サインイン
}
remote-status-org = - 組織 { $name } ({ $org })、フィンガープリント { $fingerprint }
remote-audit-verified = { $events } 件の監査イベントと { $checkpoints } 件のチェックポイントを検証しました。
remote-change-proposed = 変更 { $kind } には 2 人目の管理者が必要です。提案 { $proposal } は 'pasejo org cosign' を待っています。
remote-change-recorded = { $kind } をメンバーシップログに記録しました。
remote-device-active = このデバイスは { $org } で有効です。
remote-device-pending = このデバイスは { $org } で承認待ちです。管理者または承認者に識別子 { $device } とフィンガープリント { $fingerprint } を伝えてください。
remote-device-fingerprint = このデバイスのフィンガープリントは { $fingerprint } です。
remote-logged-in = { $url } に { $email } としてサインインしました。
remote-logged-out = { $url } からサインアウトしました。
remote-login-visit = ブラウザーで { $url } を開き、コード { $code } を確認してください。
remote-no-actions = 保留中のアクションはありません。
remote-no-proposals = 未処理の提案はありません。
remote-no-servers = サーバーが登録されていません。
remote-no-stores = このデバイスから見える組織のストアはありません。
remote-org-created = 組織 { $name } を識別子 { $org }、フィンガープリント { $fingerprint } で作成しました。
remote-rotation-access-continues = { $user } は引き続きストアにアクセスできます。
remote-rotation-nothing = { $user } が取得したもので、まだローテーションが必要なものはありません。
remote-settings-unchanged = 設定は既にこれらの値です。
remote-store-added = リモートストア { $label } を '{ $name }' として登録しました。
remote-store-created = リモートストア '{ $name }' を識別子 { $store } で作成しました。
remote-store-migrated = '{ $name }' の { $count } 件のエントリをサーバーにコピーしました。ストアは今後リモートのコピーを参照します。ファイル { $path } は変更されていません。
remote-store-recovered = { $label } の { $count } 件のエントリを { $path } に復旧しました。
remote-store-rekeyed = { $count } 件のエントリを現在の鍵で書き直しました。

## Remote administration
error-remote-emergency-unregistered = 緊急サインインには { $url } に登録済みのデバイスが必要です。
error-remote-unknown-review = 識別子 { $review } のアクセスレビューはありません。
error-remote-not-under-review = { $principal } はレビュー { $review } に権限を持っていません。
remote-emergency-logged-in = 緊急ウィンドウを通じて { $email } として { $url } にサインインしました（{ $expires } まで）。
remote-scim-token-created = SCIM トークン { $id } を作成しました。以下に一度だけ表示され、再表示はできません。
remote-scim-token-line = { $id }  { $created }
remote-no-scim-tokens = 有効な SCIM トークンはありません。
remote-scim-token-revoked = SCIM トークン { $id } を失効させました。
remote-review-opened = ストア { $store } のアクセスレビュー { $review } を開始しました。期限は { $due } です。
remote-review-line = { $review }  { $store }  { $due }  { $state }
remote-review-item = - { $principal }  { $role }  { $decision }
remote-review-state-open = 進行中
remote-review-state-complete = 完了
remote-review-decision-keep = 維持
remote-review-decision-revoke = 取り消し
remote-review-decision-pending = 未決定
remote-no-reviews = アクセスレビューはありません。
remote-review-decided = レビュー { $review } で { $principal } に「{ $decision }」を記録しました。
remote-review-revokes-pending = 取り消しの決定は、組織またはストアの管理者が 'pasejo org actions --run' を実行したときに反映されます。
