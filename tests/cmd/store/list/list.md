```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo store add --path path/to/another --name another --default
Store 'another' added at '[CWD]/path/to/another'
Store 'another' is now the default

$ pasejo store add --path somewhere/else --name other
Store 'other' added at '[CWD]/somewhere/else'

$ pasejo store list
something: [CWD]/store
another: [CWD]/path/to/another (default)
other: [CWD]/somewhere/else

```
