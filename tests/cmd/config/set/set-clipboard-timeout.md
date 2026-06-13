```
$ pasejo config set clipboard-timeout 123

$ pasejo config get clipboard-timeout
123

$ pasejo config set clipboard-timeout 1

$ pasejo config get clipboard-timeout
1

$ pasejo config set clipboard-timeout -- -1
? 1
Error: Invalid value '-1' for option 'clipboard-timeout': expected a non-negative integer

$ pasejo config get clipboard-timeout
1

```
