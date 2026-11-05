```
$ pasejo browser help install-manifest
Install the native-messaging manifest for one or more browsers

Usage: pasejo browser install-manifest [OPTIONS] --browser <BROWSERS>

Options:
      --browser <BROWSERS>            Browser to install the manifest for. Repeatable [possible values: firefox, chrome, chromium, brave, edge, vivaldi, opera]
      --extension-id <EXTENSION_IDS>  Extension id allowed to connect to the host. Repeatable. Required for chromium-family browsers; Firefox defaults to the bundled extension id
      --print                         Print the resulting manifest JSON to stdout instead of writing it to disk. Does not create the wrapper script
      --binary <BINARY>               Override the path written into the manifest. When set, no wrapper script is installed and the supplied path is referenced verbatim — useful for callers that maintain their own launcher
      --host-name <HOST_NAME>         Override the native-messaging host name. Defaults to `wtf.metio.pasejo`; only change when integrating with a non-default extension build
      --env <ENVS>                    Environment variable in `KEY=VALUE` form to bake into the generated wrapper script. Repeatable. Use this to pin `SSH_AUTH_SOCK`, `PASEJO_CONFIG`, `PASEJO_DEFAULT_STORE`, or anything else the browser-spawned host won't otherwise inherit. Has no effect with `--print` (the wrapper isn't generated then) or with `--binary` (the wrapper isn't generated then)
  -v, --verbose...                    Increase logging verbosity
  -q, --quiet...                      Decrease logging verbosity
  -h, --help                          Print help

```

```
$ pasejo browser install-manifest --help
Install the native-messaging manifest for one or more browsers

Usage: pasejo browser install-manifest [OPTIONS] --browser <BROWSERS>

Options:
      --browser <BROWSERS>            Browser to install the manifest for. Repeatable [possible values: firefox, chrome, chromium, brave, edge, vivaldi, opera]
      --extension-id <EXTENSION_IDS>  Extension id allowed to connect to the host. Repeatable. Required for chromium-family browsers; Firefox defaults to the bundled extension id
      --print                         Print the resulting manifest JSON to stdout instead of writing it to disk. Does not create the wrapper script
      --binary <BINARY>               Override the path written into the manifest. When set, no wrapper script is installed and the supplied path is referenced verbatim — useful for callers that maintain their own launcher
      --host-name <HOST_NAME>         Override the native-messaging host name. Defaults to `wtf.metio.pasejo`; only change when integrating with a non-default extension build
      --env <ENVS>                    Environment variable in `KEY=VALUE` form to bake into the generated wrapper script. Repeatable. Use this to pin `SSH_AUTH_SOCK`, `PASEJO_CONFIG`, `PASEJO_DEFAULT_STORE`, or anything else the browser-spawned host won't otherwise inherit. Has no effect with `--print` (the wrapper isn't generated then) or with `--binary` (the wrapper isn't generated then)
  -v, --verbose...                    Increase logging verbosity
  -q, --quiet...                      Decrease logging verbosity
  -h, --help                          Print help

```

```
$ pasejo browser install-manifest -h
Install the native-messaging manifest for one or more browsers

Usage: pasejo browser install-manifest [OPTIONS] --browser <BROWSERS>

Options:
      --browser <BROWSERS>            Browser to install the manifest for. Repeatable [possible values: firefox, chrome, chromium, brave, edge, vivaldi, opera]
      --extension-id <EXTENSION_IDS>  Extension id allowed to connect to the host. Repeatable. Required for chromium-family browsers; Firefox defaults to the bundled extension id
      --print                         Print the resulting manifest JSON to stdout instead of writing it to disk. Does not create the wrapper script
      --binary <BINARY>               Override the path written into the manifest. When set, no wrapper script is installed and the supplied path is referenced verbatim — useful for callers that maintain their own launcher
      --host-name <HOST_NAME>         Override the native-messaging host name. Defaults to `wtf.metio.pasejo`; only change when integrating with a non-default extension build
      --env <ENVS>                    Environment variable in `KEY=VALUE` form to bake into the generated wrapper script. Repeatable. Use this to pin `SSH_AUTH_SOCK`, `PASEJO_CONFIG`, `PASEJO_DEFAULT_STORE`, or anything else the browser-spawned host won't otherwise inherit. Has no effect with `--print` (the wrapper isn't generated then) or with `--binary` (the wrapper isn't generated then)
  -v, --verbose...                    Increase logging verbosity
  -q, --quiet...                      Decrease logging verbosity
  -h, --help                          Print help

```
