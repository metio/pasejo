# `pasejo hook`

The `hook` subcommand manages the shell commands pasejo runs to synchronize a store with a remote — pull hooks run before a store is read, push hooks after it is written. Its help output looks like this:

```console
$ pasejo hook --help
Manage hooks

Usage: pasejo hook [OPTIONS] <COMMAND>

Commands:
  set   Set hook commands to a store or globally to all stores
  get   Get configured hook commands
  run   Run configured hook commands
  help  Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`hook set` records pull and/or push commands either globally (`--global`, applied to every store) or on a single store. By default the supplied commands replace the existing list; `--prepend` and `--append` add to it instead. Global commands run before per-store commands.

`hook get` prints the pull and push commands currently configured for the selected scope.

`hook run` executes the configured hooks on demand. `--pull` and `--push` select which side to run (passing neither runs nothing and prints a warning); `--all` runs them across every configured store rather than just the selected one.

## Execution model

Hook commands are **not** run through a shell. Each command string is tokenized with `shlex` and the first token is executed directly as a program with the rest as its arguments, so there is no glob expansion, no `&&`/`|`/`;` chaining, and no environment-variable interpolation — write one program invocation per command and add multiple commands for multiple steps. The working directory is the store file's parent directory.

The token `%p` is the store path placeholder: a standalone `%p` token is passed as a single argument carrying the raw path (so paths containing spaces, quotes, or shell metacharacters cannot break out of their argument), while `%p` inside a larger token (e.g. `--git-dir=%p/.git`) is string-substituted and requires the store path to be valid UTF-8.

Pull and push hooks are **throttled** per store: after a successful run pasejo stamps a timestamp in its data directory and skips the same side until the configured interval elapses. The timestamp is written only on success, so a failed hook is retried on the next invocation rather than silently suppressed for the rest of the interval. `hook run` bypasses the throttle and always executes. Set `PASEJO_DISABLE_HOOK_THROTTLING` to disable throttling entirely.
