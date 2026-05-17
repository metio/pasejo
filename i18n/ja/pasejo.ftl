# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## 受信者

recipient-added = 受信者 '{ $public_key }' を追加しました
recipient-removed = 受信者 '{ $public_key }' を削除しました
recipient-does-not-exist-ignored = 受信者 '{ $public_key }' はストアに存在しません - 無視します

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

## フックの実行

execute-pull-hooks = ストア '{ $store_name }' のプルフックを実行しています
execute-push-hooks = ストア '{ $store_name }' のプッシュフックを実行しています

## マージ競合

merge-conflict-recipient-names = 公開鍵 '{ $public_key }' の受信者でマージ競合: 名前 '{ $first_name }' と '{ $second_name }' が異なります
merge-conflict-recipient-removed-and-renamed = 公開鍵 '{ $public_key }' の受信者でマージ競合: 一方のバージョンでは削除され、もう一方では '{ $new_name }' に名前変更されました
merge-conflict-values = '{ $secret_path }' の { $value_type } でマージ競合: 二つのバージョンで値が異なります
merge-conflict-removed-and-modified = '{ $secret_path }' の { $value_type } でマージ競合: 一方のバージョンでは削除され、もう一方では変更されました

## リスト出力 (stdout)

list-global-identity = グローバル: { $identity_file }
list-store-identity = ストア: { $identity_file }
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
error-no-secret-found = '{ $secret_path }' にシークレットが見つかりません
error-not-allowed-to-remove-secret = { $secret_path } のシークレットの削除は許可されていません。上書きするには --force を使用してください。
error-secret-does-not-exist-for-edit = { $secret_path } にシークレットが存在しません。作成するには 'pasejo secret add' を使用してください。
error-one-time-password-already-exists = ワンタイムパスワードは既に { $password_path } に存在します。上書きするには --force を使用してください。
error-not-allowed-to-remove-one-time-password = { $password_path } のワンタイムパスワードの削除は許可されていません。上書きするには --force を使用してください。
error-no-one-time-password-found = '{ $password_path }' にワンタイムパスワードが見つかりません
error-no-store-in-configuration = 設定にストアが見つかりません。最初に 'pasejo store add ...' を実行して追加してください
error-decrypt-requires-yes-i-know = 復号を拒否しました: すべてのシークレットが平文で stdout に出力されます。確認のため --yes-i-know を指定してください。
error-no-store-or-global = 設定にストアが見つからず、--global フラグも指定されていません。最初に 'pasejo store add ...' を実行して追加してください
error-store-name-already-exists = ストア名は既に存在します。別の名前を使用してください。
error-store-path-is-directory = ディレクトリをストアパスとして使用できません。ファイルパスを使用してください。
error-cannot-create-store-path = ストアパスを作成できません。パスを確認してやり直してください。
error-cannot-get-store-parent = ストアパスの親ディレクトリを取得できません。パスを確認してやり直してください。
error-cannot-identify-store = ストアを識別できません。デフォルトストアを設定するか、--store でストアを指定するか、--global でアイデンティティをグローバルに設定してください。
error-no-identity-files-to-decrypt = 復号するアイデンティティファイルがありません。ストアの初期化を完了するには、少なくとも 1 つのアイデンティティを追加してください。
error-invalid-ssh-public-key-format = 無効な SSH 公開鍵形式
error-username-empty = ユーザー名は空であってはなりません
error-username-contains-dotdot = 無効なユーザー名 '{ $username }': '..' を含めてはなりません
error-username-invalid-character = 無効なユーザー名 '{ $username }': ASCII の英字または数字で始まり、ASCII の英字、数字、'-'、'_'、'.' のみを含む必要があります
error-no-public-key-found-in-file = '{ $filename }' に公開鍵が見つかりません
error-no-public-key-source = 公開鍵のソースを少なくとも 1 つ指定する必要があります
error-no-qrcode-found = '{ $qrcode }' に QR コードが見つかりません
error-failed-to-decode-qrcode = '{ $qrcode }' の QR コードのデコードに失敗しました
error-cannot-determine-store-name = ストア名を特定できません
error-cannot-parse-hook-command = コマンドを解析できません: { $command }
error-empty-hook-command = 空のフックコマンド: { $command }
error-failed-to-run-hook = フック { $command } の実行に失敗しました
error-hook-failed-no-detail = フック { $command } が失敗しました (終了 { $exit })
error-hook-failed-with-detail = フック { $command } が失敗しました (終了 { $exit }): { $detail }
error-cannot-determine-store-parent-path = ストアパス { $path } の親を特定できません
error-store-path-not-utf8 = トークン { $token } の %p を置換できません: ストアパス { $path } は有効な UTF-8 ではありません
error-merge-conflict-recipients = 受信者でマージ競合が検出されました。手動で解決してください。
error-recipient-not-found-in-store = 受信者がストアに見つかりません
error-cannot-decrypt-common-ancestor-store = 共通祖先ストアを復号できません
error-cannot-decrypt-current-version-store = 現在のバージョンのストアを復号できません
error-cannot-decrypt-other-version-store = 他のバージョンのストアを復号できません
error-cannot-read-file = '{ $path }' のファイルを読み取れません
error-downloading-public-key-failed = { $provider } からの公開鍵のダウンロードに失敗しました
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
error-could-not-open-store-for-lock = ロック用にストアファイルを開けませんでした: { $path }
error-could-not-acquire-store-lock = ストアファイルのロックを取得できませんでした: { $path }
error-cannot-encrypt-store = ストアを暗号化できません
error-failed-to-run-command = コマンド { $binary } の実行に失敗しました
error-command-exited-with = コマンド { $binary } は { $exit } で終了しました
