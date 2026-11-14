# `pasejo exec`

The `exec` command runs another command with values from secrets added to its environment. Its help output looks like this:

```console
$ pasejo exec --help
Run a command with values from secrets in its environment

Usage: pasejo exec [OPTIONS] <COMMAND>...

Arguments:
  <COMMAND>...  The command to run, followed by its arguments

Options:
  -s, --store <STORE>
          Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -e, --env <VARIABLE=SECRET>
          Set VARIABLE to the first line of SECRET, the conventional place of its password. Repeatable
      --env-field <VARIABLE=SECRET=FIELD>
          Set VARIABLE to the value of the FIELD field of SECRET. Repeatable
      --redact
          Replace the values in the command's output with ${VARIABLE}
  -v, --verbose...
          Increase logging verbosity
  -q, --quiet...
          Decrease logging verbosity
  -h, --help
          Print help

```

`--env VARIABLE=SECRET` sets the variable to the first line of the secret, where pasejo keeps the password by convention. `--env-field VARIABLE=SECRET=FIELD` sets it to the value of a `FIELD: value` line instead, without surrounding whitespace; everything between the first and the last `=` is the secret's path, so a path may contain `=` and a field key may not. Both flags repeat, a variable may be assigned only once, and every value comes from the store `--store` selects. When a secret, line or field is missing, pasejo names the variable and exits without starting the command.

```sh
pasejo exec --env GITHUB_TOKEN=api/github.com/token --env-field DB_USER=db/prod=username -- ./deploy.sh
```

The values are decrypted once, before the command starts, and pull hooks run as for any other read. On Unix, pasejo then replaces itself with the command, so the exit status, signals and terminal belong to the command alone. On Windows, pasejo runs the command as a child and exits with its exit code. In both cases the values stay in the command's environment while it runs: every process it starts inherits them, and on Linux any process of the same user can read them from `/proc/<pid>/environ`.

`--redact` keeps pasejo between the command and the terminal and replaces each value in the command's stdout and stderr with `${VARIABLE}`, which helps when sharing a screen or a log. Values shorter than 3 characters are left alone, with a warning. Output that begins like a value is held back until the next output, or the end of the stream, shows whether it is one. The replacement matches the exact bytes of each value, so a value the command prints in another form (base64, URL-encoded, split across lines) passes through unchanged. pasejo exits with the command's exit code, or with 128 plus the signal number when a signal ended the command. A Ctrl-C reaches pasejo as well and ends it, so output the command writes afterwards is discarded rather than shown unredacted. Stdin stays connected to the terminal.

[`secret resolve`](./pasejo-cmd-secret.md) offers the same lookup to other programs: it reads references as JSON and prints the values as JSON.
