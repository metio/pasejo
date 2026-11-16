```
$ pasejo org review help decide
Decide whether a grant under review stays

Usage: pasejo org review decide [OPTIONS] <REVIEW> <PRINCIPAL> <DECISION>

Arguments:
  <REVIEW>     The review's identifier
  <PRINCIPAL>  The member's email address, or the team's name, whose grant is decided
  <DECISION>   Whether the grant stays [possible values: keep, revoke]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

```
$ pasejo org review decide --help
Decide whether a grant under review stays

Usage: pasejo org review decide [OPTIONS] <REVIEW> <PRINCIPAL> <DECISION>

Arguments:
  <REVIEW>     The review's identifier
  <PRINCIPAL>  The member's email address, or the team's name, whose grant is decided
  <DECISION>   Whether the grant stays [possible values: keep, revoke]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

```
$ pasejo org review decide -h
Decide whether a grant under review stays

Usage: pasejo org review decide [OPTIONS] <REVIEW> <PRINCIPAL> <DECISION>

Arguments:
  <REVIEW>     The review's identifier
  <PRINCIPAL>  The member's email address, or the team's name, whose grant is decided
  <DECISION>   Whether the grant stays [possible values: keep, revoke]

Options:
  -o, --org <ORG>   Name or identifier of the organization. Defaults to the only registered organization
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```
