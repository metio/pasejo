# Enable shell completions

You can enable shell completions for your shell of choice using the `COMPLETE` environment variable. For example, to create a bash completion file, run:

```shell
$ COMPLETE=bash pasejo > ${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/pasejo
```

The supported shells are:
- `bash`
- `elvish`
- `fish`
- `powershell`
- `zsh`

Check their respective documentation on where to install completion files.
