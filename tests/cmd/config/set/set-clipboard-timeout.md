```
$ pasejo config set clipboard-timeout 123

$ pasejo config get clipboard-timeout
123

$ pasejo config set clipboard-timeout 1

$ pasejo config get clipboard-timeout
1

$ pasejo config set clipboard-timeout -- -1
? 1
Error: invalid digit found in string

$ pasejo config get clipboard-timeout
1

```
