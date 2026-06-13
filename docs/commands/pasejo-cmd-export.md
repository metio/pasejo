# `pasejo export`

The `export` subcommand writes the secrets of a store out in the import format of another password manager. Its help output looks like this:

```console
$ pasejo export --help
Export passwords

Usage: pasejo export [OPTIONS] <COMMAND>

Commands:
  bitwarden  Export all passwords of a store in Bitwarden JSON format
  help       Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`export bitwarden` decrypts the selected store and serializes every secret as Bitwarden import JSON. **The output is the entire vault in cleartext** — the command prints a `Warning: the export contains every secret in plaintext.` on stderr regardless of where the output goes. Pass `--organization-id` (optionally with `--collection-id` / `--collection-name`) to emit the organization format instead of the personal format; without it the personal format is produced.

By default the JSON is written to stdout. Redirecting it with the shell (`> backup.json`) creates the file at your umask default — typically world- or group-readable — and leaves the command in your shell history. Prefer `--out <FILE>`, which writes the file mode `0600` (Unix) via an atomic sibling-tempfile-and-rename, so the cleartext never lands at the umask default and never passes through the shell. Delete the file as soon as the import is done.

`--username-keys` and `--uri-keys` select which multiline fields of a secret are mapped to the Bitwarden `username` and `uri` fields; they default to the same key lists the browser integration uses (`login email username` and `uri url link site`). When a secret body repeats a key, the **last** occurrence wins and earlier values are dropped from the export — keep one value per key if you need all of them preserved.
