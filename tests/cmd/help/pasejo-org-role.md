```
$ pasejo org help role
Change a member's role

Usage: pasejo org role [OPTIONS] <USER> <ROLE>

Arguments:
  <USER>  The member's email address, or `machine:NAME` for a machine
  <ROLE>  The new role; `none` removes the member [possible values: admin, approver, auditor, member, none]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

```
$ pasejo org role --help
Change a member's role

Usage: pasejo org role [OPTIONS] <USER> <ROLE>

Arguments:
  <USER>  The member's email address, or `machine:NAME` for a machine
  <ROLE>  The new role; `none` removes the member [possible values: admin, approver, auditor, member, none]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

```
$ pasejo org role -h
Change a member's role

Usage: pasejo org role [OPTIONS] <USER> <ROLE>

Arguments:
  <USER>  The member's email address, or `machine:NAME` for a machine
  <ROLE>  The new role; `none` removes the member [possible values: admin, approver, auditor, member, none]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```
