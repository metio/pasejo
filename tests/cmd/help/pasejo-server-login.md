```
$ pasejo server help login
Register this installation as a device of a server and sign in

Usage: pasejo server login [OPTIONS] <URL>

Arguments:
  <URL>  The server's base URL

Options:
      --label <LABEL>
          The device label shown to approvers. Defaults to the host name
      --recipient <RECIPIENT>
          The age recipient the server encrypts store keys to. Defaults to the recipient of the first global X25519 identity
      --workload-token <WORKLOAD_TOKEN>
          Sign in as a machine with the workload token in this file, read again whenever the session expires
      --emergency
          Sign in with this device's key while the server's emergency window is open, without the identity provider
  -v, --verbose...
          Increase logging verbosity
  -q, --quiet...
          Decrease logging verbosity
  -h, --help
          Print help

```

```
$ pasejo server login --help
Register this installation as a device of a server and sign in

Usage: pasejo server login [OPTIONS] <URL>

Arguments:
  <URL>  The server's base URL

Options:
      --label <LABEL>
          The device label shown to approvers. Defaults to the host name
      --recipient <RECIPIENT>
          The age recipient the server encrypts store keys to. Defaults to the recipient of the first global X25519 identity
      --workload-token <WORKLOAD_TOKEN>
          Sign in as a machine with the workload token in this file, read again whenever the session expires
      --emergency
          Sign in with this device's key while the server's emergency window is open, without the identity provider
  -v, --verbose...
          Increase logging verbosity
  -q, --quiet...
          Decrease logging verbosity
  -h, --help
          Print help

```

```
$ pasejo server login -h
Register this installation as a device of a server and sign in

Usage: pasejo server login [OPTIONS] <URL>

Arguments:
  <URL>  The server's base URL

Options:
      --label <LABEL>
          The device label shown to approvers. Defaults to the host name
      --recipient <RECIPIENT>
          The age recipient the server encrypts store keys to. Defaults to the recipient of the first global X25519 identity
      --workload-token <WORKLOAD_TOKEN>
          Sign in as a machine with the workload token in this file, read again whenever the session expires
      --emergency
          Sign in with this device's key while the server's emergency window is open, without the identity provider
  -v, --verbose...
          Increase logging verbosity
  -q, --quiet...
          Decrease logging verbosity
  -h, --help
          Print help

```
