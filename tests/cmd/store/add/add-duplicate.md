```
$ pasejo store add --path store --name same
Store 'same' added at '[CWD]/store'

$ pasejo store add --path different --name same
? 1
Error: Store name already exists. Please use a different name.

$ pasejo store add --path store --name different
Store 'different' added at '[CWD]/store'

```
