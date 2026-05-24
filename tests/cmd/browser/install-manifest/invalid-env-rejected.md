```
$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --env BROKEN
? 1
Error: --env entry 'BROKEN' is not in KEY=VALUE form.

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --env 1BAD=value
? 1
Error: --env name '1BAD' is not a valid POSIX environment variable name (must start with a letter or underscore and contain only letters, digits, and underscores).

```
