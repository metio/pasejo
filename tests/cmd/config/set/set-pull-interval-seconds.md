```
$ pasejo config set pull-interval-seconds 123

$ pasejo config get pull-interval-seconds
123

$ pasejo config set pull-interval-seconds 1

$ pasejo config get pull-interval-seconds
1

$ pasejo config set pull-interval-seconds -- -1
? 1
Error: invalid digit found in string

$ pasejo config get pull-interval-seconds
1

```
