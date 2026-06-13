# `pasejo identity`

The `identity` subcommand is used to manage identities used in your stores. Its help output looks like this:

```console
$ pasejo identity --help
Manage identities

Usage: pasejo identity [OPTIONS] <COMMAND>

Commands:
  add     Adds an identity either to a single store or to your global configuration file
  remove  Remove an identity
  list    List identities
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

An identity is the private age or SSH key pasejo uses to decrypt a store. `identity add` records a key file either globally (`--global`, usable by every store) or on a single store; `identity remove` drops it from the same scope. Identity paths are stored as lexical absolute paths and matched verbatim, so a symlinked or `..`-spelled path that was added one way must be removed with the same spelling. When a `remove` does not match exactly but the supplied path resolves to the same file as a registration, the error names the stored spelling (`Did you mean '…'?`) so you are not left guessing which form was recorded.

`identity list` is read-only and always prints the global identities first — even when no store is configured yet, where `identity add --global` is still valid — then the selected store's identities. It never raises the "no store or --global" error that the mutating `add`/`remove` paths use when they need a target scope. The same file may be registered both globally and on a store (adding it to one scope does not block the other); when that happens the store-scope line is flagged `(also registered globally)` so the redundancy is visible. It is harmless — a global identity already covers every store, and decryption deduplicates the key — but the per-store copy adds nothing.


