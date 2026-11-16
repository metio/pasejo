```
$ pasejo store help grant
Grant a user or team a role on a remote store

Usage: pasejo store grant [OPTIONS] <PRINCIPAL> <ROLE>

Arguments:
  <PRINCIPAL>  A member's email address, or a team's name
  <ROLE>       The role; `none` takes the grant away [possible values: reader, writer, admin, none]

Options:
      --expires <EXPIRES>  The day the grant ends, as YYYY-MM-DD in UTC
  -s, --store <STORE>      Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -v, --verbose...         Increase logging verbosity
  -q, --quiet...           Decrease logging verbosity
  -h, --help               Print help

```

```
$ pasejo store grant --help
Grant a user or team a role on a remote store

Usage: pasejo store grant [OPTIONS] <PRINCIPAL> <ROLE>

Arguments:
  <PRINCIPAL>  A member's email address, or a team's name
  <ROLE>       The role; `none` takes the grant away [possible values: reader, writer, admin, none]

Options:
      --expires <EXPIRES>  The day the grant ends, as YYYY-MM-DD in UTC
  -s, --store <STORE>      Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -v, --verbose...         Increase logging verbosity
  -q, --quiet...           Decrease logging verbosity
  -h, --help               Print help

```

```
$ pasejo store grant -h
Grant a user or team a role on a remote store

Usage: pasejo store grant [OPTIONS] <PRINCIPAL> <ROLE>

Arguments:
  <PRINCIPAL>  A member's email address, or a team's name
  <ROLE>       The role; `none` takes the grant away [possible values: reader, writer, admin, none]

Options:
      --expires <EXPIRES>  The day the grant ends, as YYYY-MM-DD in UTC
  -s, --store <STORE>      Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -v, --verbose...         Increase logging verbosity
  -q, --quiet...           Decrease logging verbosity
  -h, --help               Print help

```
