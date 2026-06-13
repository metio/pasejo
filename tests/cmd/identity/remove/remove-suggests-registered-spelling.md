Identity paths are matched verbatim, so a key registered under one spelling
cannot be removed under an equivalent-but-different one. When the supplied
file resolves (via the filesystem) to the same file as a registration, the
error names the stored spelling instead of leaving the user to guess. Here
`sub/../some-identity` is the same file as the stored `some-identity`, but a
different string, so the exact-match check misses and the hint fires.

```
$ pasejo identity add --file some-identity --global
Identity using file '[CWD]/some-identity' added

$ pasejo identity remove --file sub/../some-identity --global
? 2
error: invalid value 'sub/../some-identity' for '--file <FILE>': file does not match any known identity. Did you mean '[CWD]/some-identity'? Identity paths are matched exactly.

Usage: pasejo identity remove [OPTIONS] --file <FILE>

For more information, try '--help'.

```
