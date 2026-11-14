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
  resolve   Print values from secrets as JSON, for references read from stdin
  show      Show secret
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`secret show` prints the requested secret in cleartext to stdout — there is no acknowledgement prompt the way `store decrypt` has one, because a single secret is the intended unit of retrieval. Use `--clip` to copy it to the clipboard instead (auto-cleared after a delay), `--qrcode` to render it as a QR code, `--line N` to print just one line (1-indexed; negatives count from the end, `-1` is the last line), or `--skip-lines N` to drop the first N lines (the convention is that the password is line 1, so `--skip-lines 1` returns just the metadata), or `--field KEY` to print the value of a `KEY: value` line without surrounding whitespace. Field keys compare exactly, and when a key repeats, the last line wins. A `--line` value outside the secret's range prints nothing and exits 0 rather than erroring, so a missing line is indistinguishable from an empty one; a missing field is an error.

`secret add --multiline` and `secret edit` open the secret in `$VISUAL`, else `$EDITOR`, else `nano` (`notepad` on Windows), through a temporary file. The variable names the editor program itself and is not split into arguments, so an editor that needs flags has to be wrapped in a small script. The file is created mode `0600` (owner-only) in `$XDG_RUNTIME_DIR`, else in `/dev/shm`; on most Linux systems both are RAM-backed. When neither directory exists, as on macOS and Windows, pasejo prints a warning and uses the system temp directory, which may be stored on disk. After the editor exits, the file is overwritten with zeros and removed. This also happens when the editor exits with a non-zero status, in which case the secret is left unchanged. Trailing line breaks are removed from the edited text. Some exposures remain: an editor that saves by writing a new file and renaming it over the old one leaves the earlier content in an unlinked file that pasejo cannot overwrite; `SIGKILL`, a power loss, or closing the terminal while the editor is open skips the cleanup; and editors may write swap, backup or undo files of their own.

`secret resolve` looks up several values at once for programs that pass them on, such as a task runner that sets environment variables. It reads a JSON object from stdin that maps names, such as environment variable names, to references, and prints a JSON object that maps the same names to the values:

```sh
echo '{"TOKEN": {"secret": "api/github.com/token"}, "DB_USER": {"secret": "db/prod", "store": "work", "field": "username"}}' | pasejo secret resolve
```

```json
{"DB_USER":"alice","TOKEN":"ghp_…"}
```

A reference names its `secret` and optionally a `store`; without one, the store `--store` selects applies. It may add either `line`, counted as for `--line`, or `field`, read as for `--field`. Without either, it selects line 1, the password. The names are labels of the caller's choosing, repeated in the output and in error messages, so they need not be valid variable names. Each store is decrypted once, however many references point into it. The output is all or nothing: a malformed input, an unknown store, or a missing secret, line or field prints nothing to stdout and exits non-zero with an error that names the reference. Unlike `secret show`, a line outside the secret's range is an error here, since an empty value would pass for a real one. [`pasejo exec`](./pasejo-cmd-exec.md) uses the same lookup to run a command with the values in its environment.
