A file registered both globally and on a store is listed under each scope.
The store line is flagged `(also registered globally)` so the redundancy
is visible — the global registration already covers every store, so the
per-store copy is harmless but adds nothing.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --global
Identity using file '[CWD]/some-identity' added

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo identity list
global: [CWD]/some-identity
store: [CWD]/some-identity (also registered globally)

```
