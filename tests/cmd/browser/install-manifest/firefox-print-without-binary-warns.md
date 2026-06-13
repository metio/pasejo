Without `--binary`, the printed manifest's `path` points at a wrapper
under the data directory that `--print` does not install. The command
warns about that on stderr while keeping the JSON on stdout pipeable.
The wrapper path is environment-dependent, so it is wildcarded. trycmd
merges stderr into stdout, and the note is logged before the JSON is
printed, so the note appears first.

```
$ pasejo browser install-manifest --browser firefox --print
Note: the printed manifest references the wrapper script at '[..]wtf.metio.pasejo', which this command did not install. Pass --binary to reference an existing executable instead.
{
  "name": "wtf.metio.pasejo",
  "description": "pasejo native messaging host",
  "path": "[..]wtf.metio.pasejo",
  "type": "stdio",
  "allowed_extensions": [
    "pasejo@metio.wtf"
  ]
}

```
