```
$ pasejo secret help generate
Generate a secret and insert it into the store

Usage: pasejo secret generate [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>               Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                       Overwrite existing secrets without prompting
  -i, --inplace                     Overwrite just the password of an existing secret without prompting
  -l, --length <LENGTH>             The length of the generated passwords [default: 12]
  -n, --numbers                     Passwords are allowed to, or must if the strict is true, contain a number
  -v, --verbose...                  Increase logging verbosity
  -j, --lowercase-letters           Passwords are allowed to, or must if the strict is true, contain a lowercase letter
  -q, --quiet...                    Decrease logging verbosity
  -u, --uppercase-letters           Passwords are allowed to, or must if the strict is true, contain an uppercase letter
  -y, --symbols                     Passwords are allowed to, or must if the strict is true, contain a symbol
  -w, --spaces                      Passwords are allowed to, or must if the strict is true, contain a space
  -e, --exclude-similar-characters  Whether to exclude similar characters, iI1loO0"'`|`
  -t, --strict                      Whether the password rules are strict
  -h, --help                        Print help

```

```
$ pasejo secret generate --help
Generate a secret and insert it into the store

Usage: pasejo secret generate [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>               Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                       Overwrite existing secrets without prompting
  -i, --inplace                     Overwrite just the password of an existing secret without prompting
  -l, --length <LENGTH>             The length of the generated passwords [default: 12]
  -n, --numbers                     Passwords are allowed to, or must if the strict is true, contain a number
  -v, --verbose...                  Increase logging verbosity
  -j, --lowercase-letters           Passwords are allowed to, or must if the strict is true, contain a lowercase letter
  -q, --quiet...                    Decrease logging verbosity
  -u, --uppercase-letters           Passwords are allowed to, or must if the strict is true, contain an uppercase letter
  -y, --symbols                     Passwords are allowed to, or must if the strict is true, contain a symbol
  -w, --spaces                      Passwords are allowed to, or must if the strict is true, contain a space
  -e, --exclude-similar-characters  Whether to exclude similar characters, iI1loO0"'`|`
  -t, --strict                      Whether the password rules are strict
  -h, --help                        Print help

```

```
$ pasejo secret generate -h
Generate a secret and insert it into the store

Usage: pasejo secret generate [OPTIONS] <SECRET_PATH>

Arguments:
  <SECRET_PATH>  The path of the secret within the selected store

Options:
  -s, --store <STORE>               Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                       Overwrite existing secrets without prompting
  -i, --inplace                     Overwrite just the password of an existing secret without prompting
  -l, --length <LENGTH>             The length of the generated passwords [default: 12]
  -n, --numbers                     Passwords are allowed to, or must if the strict is true, contain a number
  -v, --verbose...                  Increase logging verbosity
  -j, --lowercase-letters           Passwords are allowed to, or must if the strict is true, contain a lowercase letter
  -q, --quiet...                    Decrease logging verbosity
  -u, --uppercase-letters           Passwords are allowed to, or must if the strict is true, contain an uppercase letter
  -y, --symbols                     Passwords are allowed to, or must if the strict is true, contain a symbol
  -w, --spaces                      Passwords are allowed to, or must if the strict is true, contain a space
  -e, --exclude-similar-characters  Whether to exclude similar characters, iI1loO0"'`|`
  -t, --strict                      Whether the password rules are strict
  -h, --help                        Print help

```
