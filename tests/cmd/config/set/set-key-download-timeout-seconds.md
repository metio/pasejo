```
$ pasejo config set key-download-timeout-seconds 5

$ pasejo config get key-download-timeout-seconds
5

$ pasejo config set key-download-timeout-seconds 60

$ pasejo config get key-download-timeout-seconds
60

$ pasejo config set key-download-timeout-seconds -- -1
? 1
Error: invalid digit found in string

$ pasejo config get key-download-timeout-seconds
60

```
