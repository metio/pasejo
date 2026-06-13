# `pasejo recipient`

The `recipient` subcommand is used to manage recipients. Its help output looks like this:

```console
$ pasejo recipient --help
Manage recipients

Usage: pasejo recipient [OPTIONS] <COMMAND>

Commands:
  add     Adds a recipient
  remove  Remove a recipient
  list    Lists all recipients
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

A recipient is an age or SSH **public** key that a store is encrypted to; everyone whose key is a recipient can decrypt it. `recipient add` takes a key inline (`--public-key`), from a file (`--file`), or by fetching a user's published keys from `--codeberg`, `--github`, or `--gitlab` (all the keys on that account are added, not just the first). Adding or removing a recipient re-encrypts the whole store to the new recipient set.

`recipient remove` refuses to remove the **last** recipient: a store with no recipients could never be decrypted again, so that removal is rejected rather than silently orphaning the data. Remember that removing a recipient does not rotate the secrets — anyone who already had a copy of the ciphertext and the matching key can still read the secrets that existed before removal, so rotate any exposed secrets after revoking access.

The host names the username lookups hit can be overridden with the `CODEBERG_HOST`, `GITHUB_HOST`, and `GITLAB_HOST` environment variables, for self-hosted instances.


