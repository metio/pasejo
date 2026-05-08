# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Recipients

recipient-added = Recipient for '{ $public_key }' added
recipient-removed = Recipient for '{ $public_key }' removed
recipient-does-not-exist-ignored = Recipient for '{ $public_key }' does not exist in store - ignoring

## Secrets

secret-added = Added secret at '{ $secret_path }'
secret-edited = Edited secret at '{ $secret_path }'
secret-generated = Generated secret at '{ $secret_path }'
secret-copied = Copied secret from '{ $source_path }' to '{ $target_path }'
secret-moved = Moved secret from '{ $source_path }' to '{ $target_path }'
secret-removed = Removed secret at '{ $secret_path }'
secret-show-as-qrcode = Showing secret at '{ $secret_path }' as QR code
secret-show-as-text = Showing secret at '{ $secret_path }' as text
secret-copy-into-clipboard = Secret '{ $secret_path }' copied to clipboard — will be cleared in { $duration } (Ctrl-C to clear now)

## One-time passwords

one-time-password-added = Added one-time password at '{ $password_path }'
one-time-password-copied = Copied one-time password from '{ $source_path }' to '{ $target_path }'
one-time-password-moved = Moved one-time password from '{ $source_path }' to '{ $target_path }'
one-time-password-removed = Removed one-time password at '{ $password_path }'
one-time-password-show = Showing one-time password at '{ $password_path }'
one-time-password-copy-into-clipboard = One-time password '{ $password_path }' copied to clipboard — will be cleared in { $duration } (Ctrl-C to clear now)

## Identities

identity-added = Identity using file '{ $identity_file }' added
identity-removed = Identity using file '{ $identity_file }' removed
no-identities-exist-yet = There are no identities in the store '{ $store_name }' yet. Please add one using 'pasejo identity add ...'

## Stores

store-add-success = Store '{ $store_name }' added at '{ $store_path }'
store-set-default = Store '{ $store_name }' is now the default
store-remove-success = Store '{ $store_name }' removed

## Hook execution

execute-pull-hooks = Executing pull hooks for store '{ $store_name }'
execute-push-hooks = Executing push hooks for store '{ $store_name }'

## Merge conflicts

merge-conflict-recipient-names = Merge conflict for recipient with public key '{ $public_key }': names '{ $first_name }' and '{ $second_name }' differ
merge-conflict-recipient-removed-and-renamed = Merge conflict for recipient with public key '{ $public_key }': recipient was removed in one version and renamed to '{ $new_name }' in the other
merge-conflict-values = Merge conflict for { $value_type } at '{ $secret_path }': values differ in the two versions
merge-conflict-removed-and-modified = Merge conflict for { $value_type } at '{ $secret_path }': { $value_type } was removed in one version and modified in the other

## List output (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = store: { $identity_file }
list-global-pull-hook = global pull: { $command }
list-global-push-hook = global push: { $command }
list-store-pull-hook = store pull: { $command }
list-store-push-hook = store push: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (default)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## Clipboard helpers

clipboard-read-for-compare-failed = Failed to read clipboard for compare: { $error }
clipboard-ctrlc-handler-install-failed = Failed to install Ctrl-C handler: { $error }. Clipboard will only clear after the configured timeout.
clipboard-clear-failed = Failed to clear clipboard: { $error }
clipboard-manual-clear-required = Clipboard could not be cleared automatically — please clear it manually now.
clipboard-notification-dispatch-failed = Failed to show clipboard-cleared notification: { $error }
clipboard-drop-clear-failed = Failed to clear clipboard during cleanup: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] Clipboard cleared (cancelled)
   *[false] Clipboard cleared
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] Clipboard left untouched (you copied something else) (cancelled)
   *[false] Clipboard left untouched (you copied something else)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] Clipboard cleared (couldn't verify contents) (cancelled)
   *[false] Clipboard cleared (couldn't verify contents)
  }
clipboard-notification-failed = { $cancelled ->
    [true] Failed to clear clipboard! Please clear it manually. (cancelled)
   *[false] Failed to clear clipboard! Please clear it manually.
  }
