A plain `identity list` (no `--global`, no store configured yet) prints
the global identities instead of erroring — listing is read-only, so
"no store" is not a failure when globals exist and apply to every store.

```
$ pasejo identity add --file some-identity --global
Identity using file '[CWD]/some-identity' added

$ pasejo identity list
global: [CWD]/some-identity

```
