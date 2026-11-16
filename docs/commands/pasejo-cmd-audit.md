# `pasejo audit`

The `audit` subcommand reads an organization's audit trail, which records who read, wrote, granted and changed what, and verifies it against the server's signed checkpoints. Its help output looks like this:

```console
$ pasejo audit --help
Read and verify an organization's audit trail

Usage: pasejo audit [OPTIONS] <COMMAND>

Commands:
  log     Print the organization's audit events
  verify  Verify the audit chain against the server's signed checkpoints
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

`audit log` prints one event per line: sequence number, time in UTC, the person who acted, the action, its outcome, the store and a detail. `--store NAME` limits the output to one registered remote store, and `--since SEQ` to events after a sequence number. Reading the trail is itself recorded.

`audit verify` recomputes the hash chain of the events and checks every checkpoint's signature with the server's checkpoint key, which pasejo pinned at the first sign-in. Organization devices co-sign checkpoints through `pasejo org actions --run`, and every co-signature must verify under the key the membership log records for its device. pasejo remembers the last verified event and checks only newer ones the next time. A server that drops, reorders or rewrites events fails verification.

Admins and auditors may read the trail.
