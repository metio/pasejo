```
$ pasejo config set key-download-timeout-seconds 5

$ pasejo config get key-download-timeout-seconds
5

$ pasejo config set key-download-timeout-seconds 60

$ pasejo config get key-download-timeout-seconds
60

$ pasejo config set key-download-timeout-seconds -- -1
? 1
Error: Invalid value '-1' for option 'key-download-timeout-seconds': expected a non-negative integer

$ pasejo config get key-download-timeout-seconds
60

```
