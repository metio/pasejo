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
store-remove-success-with-data = Store '{ $store_name }' removed and its encrypted data file deleted

## Hook execution

execute-pull-hooks = Executing pull hooks for store '{ $store_name }'
execute-push-hooks = Executing push hooks for store '{ $store_name }'
hook-run-nothing-selected = Nothing to run: pass --pull and/or --push to select which hooks to execute.
export-plaintext-warning = Warning: the export contains every secret in plaintext.
export-written-to-file = Wrote the export to '{ $path }'

## Merge conflicts

merge-conflict-recipient-names = Merge conflict for recipient with public key '{ $public_key }': names '{ $first_name }' and '{ $second_name }' differ
merge-conflict-recipient-removed-and-renamed = Merge conflict for recipient with public key '{ $public_key }': recipient was removed in one version and renamed to '{ $new_name }' in the other
merge-conflict-values = Merge conflict for { $value_type } at '{ $secret_path }': values differ in the two versions
merge-conflict-removed-and-modified = Merge conflict for { $value_type } at '{ $secret_path }': { $value_type } was removed in one version and modified in the other

## List output (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = store: { $identity_file }
list-store-identity-also-global = store: { $identity_file } (also registered globally)
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

## Prompts

prompt-enter-secret = Enter secret for { $secret_path }:
prompt-could-not-read-secret = Could not read secret for { $secret_path }
prompt-overwrite-secret = Overwrite existing secret?
prompt-remove-secret = Remove existing secret?
prompt-overwrite-one-time-password = Overwrite existing one-time password?
prompt-remove-one-time-password = Remove existing one-time password?

## Error messages

error-cannot-get-user-confirmation = Cannot get user confirmation
error-no-confirmation-from-non-terminal = Cannot get user confirmation from non-terminal input. Use --force to skip confirmation.
error-could-not-load-configuration = Could not load configuration
error-store-does-not-exist = Store with name '{ $store_name }' does not exist in configuration
error-invalid-line-number = '{ $input }' is not a valid line number
error-line-number-must-not-be-zero = Line number must not be 0. Use 1 for the first line, -1 for the last
error-invalid-count = '{ $input }' is not a valid count
error-count-must-not-be-zero = Count must not be 0. Use 1 to skip the first line
error-file-does-not-exist = The file '{ $input }' does not exist
error-secret-already-exists = Secret already exists at { $secret_path }. Use --force to overwrite.
error-secret-already-exists-overwrite-or-inplace = Secret already exists at { $secret_path }. Use --force to overwrite entirely or --inplace to modify its first line in-place.
error-no-secret-found = No secret found at '{ $secret_path }'
error-not-allowed-to-remove-secret = Not allowed to remove secret at { $secret_path }. Use --force to overwrite.
error-secret-does-not-exist-for-edit = Secret does not exist at { $secret_path }. Use 'pasejo secret add' to create it.
error-one-time-password-already-exists = One-time password already exists at { $password_path }. Use --force to overwrite.
error-not-allowed-to-remove-one-time-password = Not allowed to remove one-time password at { $password_path }. Use --force to overwrite.
error-no-one-time-password-found = No one-time password found at '{ $password_path }'
error-invalid-one-time-password-secret = The one-time password secret must be a non-empty base32 string (letters A-Z and digits 2-7)
error-invalid-otpauth-url = The otpauth URL does not describe a valid one-time password
error-hotp-counter-exhausted = The HOTP counter has reached its maximum value and cannot advance
error-no-store-in-configuration = No store found in configuration. Run 'pasejo store add ...' first to add one
error-unknown-store = Store '{ $store_name }' does not exist in the configuration. Run 'pasejo store list' to see the configured stores.
error-decrypt-requires-yes-i-know = Refusing to decrypt: this prints every secret in plaintext to stdout. Pass --yes-i-know to confirm.
error-no-store-or-global = No store found in configuration and no --global flag specified. Run 'pasejo store add ...' first to add one
error-store-name-already-exists = Store name already exists. Please use a different name.
error-store-path-is-directory = Cannot use directory as store path. Please use a file path.
error-cannot-create-store-path = Cannot create store path. Please check the path and try again.
error-cannot-get-store-parent = Cannot get parent directory of store path. Please check the path and try again.
error-cannot-identify-store = Cannot identify store. Set a default store, use --store to specify a store or use --global to set the identity globally.
error-identity-file-already-added = invalid value '{ $file }' for '--file <FILE>': file was already added as an identity
error-identity-file-not-known = invalid value '{ $file }' for '--file <FILE>': file does not match any known identity
error-identity-did-you-mean = Did you mean '{ $stored }'? Identity paths are matched exactly.
error-no-identity-files-to-decrypt = No identity files to decrypt. Add at least one identity to complete store initialization.
error-invalid-ssh-public-key-format = Invalid SSH public key format
error-username-empty = Username must not be empty
error-username-contains-dotdot = Invalid username '{ $username }': must not contain '..'
error-username-invalid-character = Invalid username '{ $username }': must start with an ASCII letter or digit and contain only ASCII letters, digits, '-', '_' and '.'
error-no-public-key-found-in-file = No public key found in '{ $filename }'
error-no-public-keys-for-user = { $provider } user '{ $username }' has no public SSH keys to add as recipients
error-no-public-key-source = You must specify at least one source for a public key
error-no-qrcode-found = No QR code found in '{ $qrcode }'
error-failed-to-decode-qrcode = Failed to decode QR code in '{ $qrcode }'
error-qr-sandbox-failed = Failed to run QR code decoder in sandbox
error-qr-sandbox-child-signal = QR code decoder terminated by signal { $signal }
error-qr-sandbox-not-enforced = QR code decoder sandbox could not be enforced; running kernel lacks Landlock support
error-cannot-determine-store-name = Cannot determine store name
error-cannot-parse-hook-command = Cannot parse command: { $command }
error-empty-hook-command = Empty hook command: { $command }
error-failed-to-run-hook = Failed to run hook { $command }
error-hook-failed-no-detail = hook { $command } failed (exit { $exit })
error-hook-failed-with-detail = hook { $command } failed (exit { $exit }): { $detail }
error-cannot-determine-store-parent-path = Cannot determine parent of store path { $path }
error-store-path-not-utf8 = Cannot substitute %p in token { $token }: store path { $path } is not valid UTF-8
error-merge-conflict-recipients = Merge conflict detected in recipients. Please resolve manually.
error-merge-conflict = Merge conflict detected in { $value_type }s. Please resolve manually.
error-recipient-not-found-in-store = Recipient not found in the store
error-cannot-remove-last-recipient = Cannot remove the last recipient from the store. Please add a new recipient before removing this one.
error-cannot-decrypt-common-ancestor-store = Cannot decrypt common ancestor store
error-cannot-decrypt-current-version-store = Cannot decrypt current version store
error-cannot-decrypt-other-version-store = Cannot decrypt other version store
error-cannot-read-file = Cannot read file at '{ $path }'
error-downloading-public-key-failed = Downloading public key from { $provider } failed
error-download-user-not-found = { $provider } has no user '{ $username }'
error-download-rate-limited = { $provider } rate-limited the request; wait a moment and try again
error-download-server-error = { $provider } returned an error (HTTP { $status })
error-download-timeout = Downloading public key from { $provider } timed out; check your connection or raise key-download-timeout-seconds
error-cannot-determine-parent-directory = Cannot determine parent directory of { $path }
error-failed-to-create-directory = Failed to create directory { $path }
error-failed-to-write-file = Failed to write { $path }
error-failed-to-fsync-file = Failed to fsync { $path }
error-failed-to-rename-file = Failed to rename { $from } to { $to }
error-failed-to-create-file = Failed to create { $path }
error-could-not-resolve-config-path = Could not resolve absolute path to configuration
error-could-not-determine-config-path = Could not determine configuration path
error-could-not-resolve-store-path = Could not resolve absolute store path
error-could-not-create-config-dir = Could not create configuration directory
error-could-not-migrate-legacy-config = Could not migrate legacy configuration file
error-could-not-move-file = Could not move file
error-could-not-copy-file = Could not copy file
error-could-not-remove-source-after-copy = Could not remove source file after copy
error-could-not-read-configuration = Could not read configuration
error-could-not-serialize-migrated-config = Could not serialize migrated configuration
error-could-not-store-configuration = Could not store configuration
error-could-not-load-migrated-config = Could not load migrated configuration
error-could-not-serialize-configuration = Could not serialize configuration
error-config-not-valid-toml = Configuration file at { $path } is not valid TOML
error-config-invalid-integer = Invalid value '{ $value }' for option '{ $option }': expected a non-negative integer
error-config-invalid-boolean = Invalid value '{ $value }' for option '{ $option }': expected one of true, false, 1, 0, yes, no, y, n
error-could-not-open-store-for-lock = Could not open store file for locking: { $path }
error-could-not-acquire-store-lock = Could not acquire lock on store file: { $path }
error-store-lock-wait-timed-out = Timed out waiting for the lock on store file: { $path }
error-cannot-encrypt-store = Cannot encrypt store
error-failed-to-run-command = Failed to run command { $binary }
error-command-exited-with = Command { $binary } exited with { $exit }
error-browser-host-requires-pipe = Browser host requires a piped stdin from the connecting browser extension. Run this command via your browser's native-messaging manifest, not directly from a terminal.
error-browser-store-file-missing = Store file does not exist: { $path }
error-browser-message-too-large = Message of { $length } bytes exceeds the native-messaging limit of { $limit } bytes
error-browser-response-too-large = Response of { $length } bytes exceeds the native-messaging limit of { $limit } bytes
error-browser-install-manifest-no-browser = Specify at least one --browser to install the native-messaging manifest for.
error-browser-install-manifest-needs-extension-id = Browser '{ $browser }' requires --extension-id because chromium-family extension ids are derived from the unpacked extension's public key.
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id cannot be combined with more than one --browser, because each browser needs its own extension id. Run the command once per browser.
error-browser-install-manifest-unsupported-os = Installing native-messaging manifests is not supported on '{ $os }'.
error-browser-install-manifest-registry-failed = Could not write the native-messaging registry key '{ $key }'
error-browser-install-manifest-no-home-dir = Could not determine the user's home directory.
error-browser-install-manifest-no-data-dir = Could not determine the user's data directory.
error-browser-install-manifest-no-current-exe = Could not determine the path of the running pasejo binary.
browser-install-manifest-written = Wrote '{ $browser }' native-messaging manifest to '{ $path }'
browser-install-manifest-print-wrapper-note = Note: the printed manifest references the wrapper script at '{ $path }', which this command did not install. Pass --binary to reference an existing executable instead.
error-browser-install-manifest-invalid-env = --env entry '{ $entry }' is not in KEY=VALUE form.
error-browser-install-manifest-invalid-env-name = --env name '{ $name }' is not a valid POSIX environment variable name (must start with a letter or underscore and contain only letters, digits, and underscores).
error-browser-install-manifest-invalid-env-value-windows = --env value of '{ $name }' cannot be written into the Windows wrapper: it contains a double quote or a line break.
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' is not a valid native-messaging host name for { $browser }: it must be dot-separated parts of letters, digits, and underscores, in lowercase for every browser except firefox.
