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
