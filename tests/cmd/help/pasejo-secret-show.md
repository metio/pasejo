```
$ pasejo secret help show
Show secret

Usage: pasejo secret show [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -o, --qrcode                   Toggle to display secrets as QR code
  -c, --clip                     Copy secret to clipboard
  -l, --line <LINE>              Show only the specified line (1-indexed). A negative value counts from the end: -1 is the last line, -2 the second-to-last
      --skip-lines <SKIP_LINES>  Show everything except the first N lines. Useful with the convention that the password lives on the first line: `--skip-lines 1` returns just the metadata
  -v, --verbose...               Increase logging verbosity
  -q, --quiet...                 Decrease logging verbosity
  -h, --help                     Print help

```

```
$ pasejo secret show --help
Show secret

Usage: pasejo secret show [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -o, --qrcode                   Toggle to display secrets as QR code
  -c, --clip                     Copy secret to clipboard
  -l, --line <LINE>              Show only the specified line (1-indexed). A negative value counts from the end: -1 is the last line, -2 the second-to-last
      --skip-lines <SKIP_LINES>  Show everything except the first N lines. Useful with the convention that the password lives on the first line: `--skip-lines 1` returns just the metadata
  -v, --verbose...               Increase logging verbosity
  -q, --quiet...                 Decrease logging verbosity
  -h, --help                     Print help

```

```
$ pasejo secret show -h
Show secret

Usage: pasejo secret show [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>            Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -o, --qrcode                   Toggle to display secrets as QR code
  -c, --clip                     Copy secret to clipboard
  -l, --line <LINE>              Show only the specified line (1-indexed). A negative value counts from the end: -1 is the last line, -2 the second-to-last
      --skip-lines <SKIP_LINES>  Show everything except the first N lines. Useful with the convention that the password lives on the first line: `--skip-lines 1` returns just the metadata
  -v, --verbose...               Increase logging verbosity
  -q, --quiet...                 Decrease logging verbosity
  -h, --help                     Print help

```
