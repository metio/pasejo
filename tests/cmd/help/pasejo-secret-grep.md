```
$ pasejo secret help grep
Grep for a search-string in secrets when decrypted

Usage: pasejo secret grep [OPTIONS] <SEARCH_STRING>

Arguments:
  <SEARCH_STRING>  The string to search in all secrets

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -r, --regex          Whether the search string should be used as a regular expression
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret grep --help
Grep for a search-string in secrets when decrypted

Usage: pasejo secret grep [OPTIONS] <SEARCH_STRING>

Arguments:
  <SEARCH_STRING>  The string to search in all secrets

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -r, --regex          Whether the search string should be used as a regular expression
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```

```
$ pasejo secret grep -h
Grep for a search-string in secrets when decrypted

Usage: pasejo secret grep [OPTIONS] <SEARCH_STRING>

Arguments:
  <SEARCH_STRING>  The string to search in all secrets

Options:
  -s, --store <STORE>  Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -r, --regex          Whether the search string should be used as a regular expression
  -v, --verbose...     Increase logging verbosity
  -q, --quiet...       Decrease logging verbosity
  -h, --help           Print help

```
