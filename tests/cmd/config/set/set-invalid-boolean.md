```
$ pasejo config set ignore-missing-identities true

$ pasejo config get ignore-missing-identities
true

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

$ pasejo config get ignore-missing-identities
true

$ pasejo config set clipboard-notify "True "
? 1
Error: Invalid boolean value 'True '. Expected one of: true, false, 1, 0, yes, no, y, n

```
