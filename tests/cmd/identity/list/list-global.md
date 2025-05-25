```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
info: Identity using file '[CWD]/some-identity' added

$ pasejo identity add --file some-identity --global
info: Identity using file '[CWD]/some-identity' added

$ pasejo identity list --global
[CWD]/some-identity (global)

```
