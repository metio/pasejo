# `pasejo store`

The `store` subcommand is used to manage stores. Its help output looks like this:

```console
$ pasejo store --help
Manage stores

Usage: pasejo store [OPTIONS] <COMMAND>

Commands:
  add            Adds a new store
  decrypt        Decrypt a store and print its content
  exec           Executes a command inside the directory of a store
  list           List all available stores
  merge          Merge two stores
  remove         Remove an existing store
  set-default    Mark a store as default
  create-remote  Create a store on a server and register it
  add-remote     Register a store that exists on a server
  grant          Grant a user or team a role on a remote store
  rekey          Rewrite every entry of a remote store under its current keys
  rotation       List what a user who lost access could still know
  migrate        Copy a file store to a server and register the copy in its place
  help           Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`store remove` removes a store from your local configuration. By default the encrypted store file on disk is left untouched and only the registration is dropped. Pass `--remove-data` to also delete the store file — this is irreversible, so make sure the store is backed up (or synced to a remote) before using it.

`store decrypt` prints **every** secret of a store in cleartext to stdout and therefore requires `--yes-i-know` to acknowledge that. `--store-path` decrypts a store file directly by path rather than by its configured name.

`store merge` is the three-way merge driver pasejo registers with git for store files: it takes `--common-ancestor`, `--current-version`, and `--other-version` and writes the merged result back to the `--current-version` path. It is normally invoked by git during a conflicting pull rather than by hand; when `--current-version` is the registered store file itself, the store lock is held for the merge so a concurrent CLI edit cannot race the write. A temporary copy, which is what Git passes, is merged without the lock.

`store exec` runs a command with its working directory set to the store's directory — useful for running `git` against the store's repository without `cd`-ing there yourself.

## Remote stores

A remote store lives on a pasejo server instead of in a local file. Secret names, store labels and values are encrypted on the client, so the server sees opaque identifiers only. Commands read only what they need: `secret list` reads the encrypted index, `secret show` one object, and the browser host a lookup object holding the URL and username fields. Recipients, hooks and merges do not apply to remote stores; the commands for them refuse a remote store.

`store create-remote LABEL` creates a store in an organization (see [`pasejo org`](./pasejo-cmd-org.md)) and registers it under `--name`, which defaults to the label. `store add-remote STORE --name NAME` registers a store that already exists, by the identifier `pasejo org stores` lists. It opens the store once and keeps the registration only when this device holds its keys.

`store grant PRINCIPAL ROLE` gives a member (by email address) or a team (by name) the role `reader`, `writer` or `admin` on the selected remote store, optionally until the end of the UTC day `--expires YYYY-MM-DD`. `none` takes a grant away. The command then wraps the store's keys for newly entitled devices, or rotates them when someone lost access.

`store rotation EMAIL` lists the entries a person who lost access read while they had it and that were not written since. Change those secrets at their source and write them again; `store rekey` rewrites every entry under the current keys.

`store migrate --to-remote ORG` copies a file store into a new remote store of the organization and points the registration at it. The store file is left in place.

`store remove --remove-data` on a remote store deletes its local cache directory; the store on the server stays.
