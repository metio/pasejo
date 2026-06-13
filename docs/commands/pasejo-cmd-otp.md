# `pasejo otp`

The `otp` subcommand is used to manage one-time passwords. Its help output looks like this:

```console
$ pasejo otp --help
Manage one-time passwords

Usage: pasejo otp [OPTIONS] <COMMAND>

Commands:
  add     Adds a one-time password
  remove  Remove a one-time password
  list    List one-time passwords
  show    Show a one-time password
  copy    Copy a one-time password from old-path to new-path
  move    Move a one-time password from old-path to new-path
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`otp show` prints the current one-time password (use `--clip` to copy it to the clipboard with auto-clear instead of printing). For **HOTP** entries this is a *mutating* read: HOTP codes are counter-based, so each `otp show` emits the code for the stored counter and then increments and re-encrypts the store. The store is therefore written even though you only asked to read — concurrent `otp show` calls are serialized by the store file lock so a counter cannot be skipped or reused. **TOTP** entries are time-based and read-only. `otp add --qrcode <FILE>` imports an `otpauth://` URL by decoding a QR image; `--digits` accepts 6–8 and `--period` must be at least 1.

