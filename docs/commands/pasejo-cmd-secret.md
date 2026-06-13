# `pasejo secret`

The `secret` subcommand is used to manage secrets. Its help output looks like this:

```console
$ pasejo secret --help
Manage secrets

Usage: pasejo secret [OPTIONS] <COMMAND>

Commands:
  add       Add a new secret or overwrite an existing one
  audit     Audit password strength of secrets
  copy      Copy secret from old-path to new-path
  edit      Edit an existing secret
  generate  Generate a secret and add it into the store
  grep      Grep for a search-string in secrets when decrypted
  list      List all secrets
  move      Move secret from old-path to new-path
  remove    Remove an existing secret
  show      Show secret
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`secret show` prints the requested secret in cleartext to stdout — there is no acknowledgement prompt the way `store decrypt` has one, because a single secret is the intended unit of retrieval. Use `--clip` to copy it to the clipboard instead (auto-cleared after a delay), `--qrcode` to render it as a QR code, `--line N` to print just one line (1-indexed; negatives count from the end, `-1` is the last line), or `--skip-lines N` to drop the first N lines (the convention is that the password is line 1, so `--skip-lines 1` returns just the metadata). A `--line` value outside the secret's range prints nothing and exits 0 rather than erroring, so a missing line is indistinguishable from an empty one.

`secret add` (multiline mode) and `secret edit` open the secret in your `$EDITOR` via a temporary file. That file is created mode `0600` (owner-only) in the system temp directory (`$TMPDIR`, usually `/tmp`) and removed when the editor exits, but it holds the secret in cleartext for the lifetime of the editor session. Two residual exposures follow: the cleartext may touch persistent disk if `$TMPDIR` is not a RAM-backed filesystem, and an editor killed by `SIGKILL` or a power loss skips the cleanup, leaving the file behind. To minimise both, point `$TMPDIR` at a tmpfs that is wiped on logout — on most Linux desktops `export TMPDIR="$XDG_RUNTIME_DIR"` does this — and prefer an editor that does not write swap/backup copies elsewhere.

