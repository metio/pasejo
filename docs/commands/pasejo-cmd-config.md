# `pasejo config`

The `config` subcommand is used to manage the `pasejo` configuration. It allows you to change configuration options used throughout the application. Its help output looks like this:

```console
$ pasejo config --help
Manage pasejo configuration

Usage: pasejo config [OPTIONS] <COMMAND>

Commands:
  get   Get a configuration value
  set   Set a configuration value
  help  Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

## Configuration Options

The following configuration options are available:

- `ignore-missing-identities`: If set to `true`, the application will ignore missing identities when decrypting stores. This is useful if you want to use the application without having all identities available, e.g., when using multiple Yubikeys. The default value is `true`.
- `clipboard-timeout`: The timeout in seconds for how long the application should keep secrets or one-time passwords in the clipboard. The default value is `45` seconds.
- `clipboard-notify`: If set to `true`, `pasejo` shows a desktop notification when the clipboard is cleared (or fails to clear) after a `secret show --clip` / `otp show --clip`. Set to `false` to silence the popup; stderr warnings on clear failure are still emitted regardless. The default value is `true`.
- `key-download-timeout-seconds`: The maximum time in seconds `pasejo` will wait when downloading a public key from a provider (Codeberg / GitHub / GitLab) before aborting with an error. The default value is `30` seconds.
- `pull-interval-seconds`: The interval in seconds `pasejo` will wait between calling the configured pull hooks of a store. The default value is `86400` seconds (24 hours).
- `push-interval-seconds`: The interval in seconds `pasejo` will wait between calling the configured push hooks of a store. The default value is `86400` seconds (24 hours).

## `pasejo config get`

The `pasejo config get` subcommand is used to retrieve the value of a specific configuration option. It works like this:

```console
$ pasejo config get --help
Get a configuration value

Usage: pasejo config get [OPTIONS] <OPTION>

Arguments:
  <OPTION>  Name of the configuration option to get [possible values: ignore-missing-identities, clipboard-timeout, clipboard-notify, key-download-timeout-seconds, pull-interval-seconds, push-interval-seconds]

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

### Examples

These examples show how to use the `pasejo config get` command to retrieve configuration values. Since we did not change these configuration options earlier, we are going to see their default values here.

```console
$ pasejo config get clipboard-timeout
45

$ pasejo config get ignore-missing-identities
true

```

## `pasejo config set`

The `pasejo config set` subcommand is used to set the value of a specific configuration option. It works like this:

```console
$ pasejo config set --help
Set a configuration value

Usage: pasejo config set [OPTIONS] <OPTION> <VALUE>

Arguments:
  <OPTION>  Name of the configuration option to set [possible values: ignore-missing-identities, clipboard-timeout, clipboard-notify, key-download-timeout-seconds, pull-interval-seconds, push-interval-seconds]
  <VALUE>   Value to set the configuration option to

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

### Examples

These examples show how to use the `pasejo config set` command to set configuration values. The first example sets the `clipboard-timeout` option to `60`, and the second example sets the `ignore-missing-identities` option to `false`.

```console
$ pasejo config set clipboard-timeout 60

$ pasejo config get clipboard-timeout
60

$ pasejo config set ignore-missing-identities false

$ pasejo config get ignore-missing-identities
false

```
