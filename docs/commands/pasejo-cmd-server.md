# `pasejo server`

The `server` subcommand registers this installation as a device of a pasejo server and manages its session. Its help output looks like this:

```console
$ pasejo server --help
Sign in to pasejo servers

Usage: pasejo server [OPTIONS] <COMMAND>

Commands:
  login   Register this installation as a device of a server and sign in
  logout  Sign out of a server
  status  Show the servers, organizations and sessions of this installation
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`server login URL` creates the device on first use: a random device identifier, an Ed25519 signing key kept mode `0600` in the data directory, and the age recipient the server encrypts store keys to. The recipient is `--recipient`, or else the recipient of the first global identity pasejo can derive one from (an X25519 identity, or an SSH key with a `.pub` file beside it); X25519, SSH and plugin recipients are accepted. `--label` names the device for approvers and defaults to the host name.

Without `--workload-token`, sign-in runs the device flow: pasejo prints a link and a code, and waits while you open the link, sign in with the identity provider and approve. It also prints the device's fingerprint, which an approver compares before admitting the device to an organization. With `--workload-token FILE`, pasejo signs in as a machine with the JWT in that file and reads the file again whenever the session expires, so a rotated token (a Kubernetes projected service-account token, for example) keeps working.

With `--emergency`, a device that is already registered signs in with its own signing key while the server's break-glass administrator has opened an emergency window, for example during an outage of the identity provider. The session lasts at most one hour and never outlasts the window. The window admits only devices that are active in an organization, so it creates no new trust, and a new installation cannot use it.

The first sign-in pins the server's checkpoint key. A later sign-in that sees a different key fails, because a replaced server could otherwise present a rewritten audit trail.

`server logout` ends the session on the server and removes it locally. `--forget` also removes the device's keys, cached logs and the server's registration; signing in again creates a new device, which needs approval again.

`server status` lists each registered server with its device, fingerprint and whether a session exists, followed by the organizations registered for it.

## Files

The device's files live in the data directory under `servers/<device>/`: `signing.key`, `session.json`, `orgs/<org>.json` (the verified membership log) and `stores/<store>/` (the pinned index of each remote store). Set `PASEJO_DATA_DIR` to move them.
