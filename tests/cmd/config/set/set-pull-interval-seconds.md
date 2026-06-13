```
$ pasejo config set pull-interval-seconds 123

$ pasejo config get pull-interval-seconds
123

$ pasejo config set pull-interval-seconds 1

$ pasejo config get pull-interval-seconds
1

$ pasejo config set pull-interval-seconds -- -1
? 1
Error: Invalid value '-1' for option 'pull-interval-seconds': expected a non-negative integer

$ pasejo config get pull-interval-seconds
1

```
