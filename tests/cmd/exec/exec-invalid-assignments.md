```
$ pasejo exec --env 1TOKEN=api/token -- true
? 2
error: invalid value '1TOKEN=api/token' for '--env <VARIABLE=SECRET>': '1TOKEN' is not a valid environment variable name. Use letters, digits and underscores, and do not start with a digit

For more information, try '--help'.

$ pasejo exec --env TOKEN -- true
? 2
error: invalid value 'TOKEN' for '--env <VARIABLE=SECRET>': 'TOKEN' is not of the form VARIABLE=SECRET

For more information, try '--help'.

$ pasejo exec --env-field USER=database -- true
? 2
error: invalid value 'USER=database' for '--env-field <VARIABLE=SECRET=FIELD>': 'USER=database' is not of the form VARIABLE=SECRET=FIELD

For more information, try '--help'.

$ pasejo exec --env TOKEN=a --env TOKEN=b -- true
? 1
Error: The environment variable TOKEN is assigned more than once

```
