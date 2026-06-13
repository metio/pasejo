# `pasejo store`

The `store` subcommand is used to manage stores. Its help output looks like this:

```console
$ pasejo store --help
Manage stores

Usage: pasejo store [OPTIONS] <COMMAND>

Commands:
  add          Adds a new store
  decrypt      Decrypt a store and print its content
  exec         Executes a command inside the directory of a store
  list         List all available stores
  merge        Merge two stores
  remove       Remove an existing store
  set-default  Mark a store as default
  help         Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`store remove` removes a store from your local configuration. By default the encrypted store file on disk is left untouched and only the registration is dropped. Pass `--remove-data` to also delete the store file — this is irreversible, so make sure the store is backed up (or synced to a remote) before using it.

`store decrypt` prints **every** secret of a store in cleartext to stdout and therefore requires `--yes-i-know` to acknowledge that. `--store-path` decrypts a store file directly by path rather than by its configured name.

`store merge` is the three-way merge driver pasejo registers with git for store files: it takes `--common-ancestor`, `--current-version`, and `--other-version` and writes the merged result back to the `--current-version` path. It is normally invoked by git during a conflicting pull rather than by hand; the current version's store file lock is held for the merge so a concurrent CLI edit cannot race the write.

`store exec` runs a command with its working directory set to the store's directory — useful for running `git` against the store's repository without `cd`-ing there yourself.
