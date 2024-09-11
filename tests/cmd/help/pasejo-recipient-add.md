```console
$ pasejo recipient help add
Adds a recipient

Usage: pasejo recipient add [OPTIONS] <--public-key <PUBLIC_KEY>|--file <FILE>|--codeberg <CODEBERG>|--github <GITHUB>|--gitlab <GITLAB>>

Options:
  -k, --public-key <PUBLIC_KEY>  The public key of the new recipient
  -f, --file <FILE>              Read public key of recipient from file
      --codeberg <CODEBERG>      The Codeberg username to add as recipient
      --github <GITHUB>          The GitHub username to add as recipient
      --gitlab <GITLAB>          The GitLab username to add as recipient
  -v, --verbose...               Increase logging verbosity
  -n, --name <NAME>              The name of the new recipient
  -q, --quiet...                 Decrease logging verbosity
  -p, --path <PATH>              The path to a folder or secret that should be readable by the given recipient
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help                     Print help

```

```console
$ pasejo recipient add --help
Adds a recipient

Usage: pasejo recipient add [OPTIONS] <--public-key <PUBLIC_KEY>|--file <FILE>|--codeberg <CODEBERG>|--github <GITHUB>|--gitlab <GITLAB>>

Options:
  -k, --public-key <PUBLIC_KEY>  The public key of the new recipient
  -f, --file <FILE>              Read public key of recipient from file
      --codeberg <CODEBERG>      The Codeberg username to add as recipient
      --github <GITHUB>          The GitHub username to add as recipient
      --gitlab <GITLAB>          The GitLab username to add as recipient
  -v, --verbose...               Increase logging verbosity
  -n, --name <NAME>              The name of the new recipient
  -q, --quiet...                 Decrease logging verbosity
  -p, --path <PATH>              The path to a folder or secret that should be readable by the given recipient
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help                     Print help

```

```console
$ pasejo recipient add -h
Adds a recipient

Usage: pasejo recipient add [OPTIONS] <--public-key <PUBLIC_KEY>|--file <FILE>|--codeberg <CODEBERG>|--github <GITHUB>|--gitlab <GITLAB>>

Options:
  -k, --public-key <PUBLIC_KEY>  The public key of the new recipient
  -f, --file <FILE>              Read public key of recipient from file
      --codeberg <CODEBERG>      The Codeberg username to add as recipient
      --github <GITHUB>          The GitHub username to add as recipient
      --gitlab <GITLAB>          The GitLab username to add as recipient
  -v, --verbose...               Increase logging verbosity
  -n, --name <NAME>              The name of the new recipient
  -q, --quiet...                 Decrease logging verbosity
  -p, --path <PATH>              The path to a folder or secret that should be readable by the given recipient
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -h, --help                     Print help

```
