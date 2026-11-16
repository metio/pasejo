# `pasejo org`

The `org` subcommand manages organizations on a pasejo server: members, devices, teams, settings and the recovery recipient. Every change is an entry in the organization's membership log, signed by your device and checked by every client that replays the log. Its help output looks like this:

```console
$ pasejo org --help
Manage organizations on a pasejo server

Usage: pasejo org [OPTIONS] <COMMAND>

Commands:
  actions    Show and run pending actions
  approve    Approve a pending device
  cosign     Co-sign a change proposed by another admin
  create     Create an organization with this device as its founding device
  devices    List the organization's devices and pending enrollments
  join       Join an organization, or check whether this device was approved
  proposals  List changes waiting for a second admin
  recover    Decrypt every store with the organization's recovery identity
  recovery   Manage the organization's recovery recipient
  revoke     Revoke a device
  role       Change a member's role
  settings   Change the organization's settings
  stores     List the organization's stores
  team       Manage teams
  help       Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help

```

Commands that act on one organization take `--org NAME_OR_ID`, which may be left out when only one organization is registered.

## Creating and joining

`org create NAME` creates an organization with this device as its founding device and you as its first admin. It prints the organization's identifier and fingerprint. Give both to the people who will join, over a channel other than the server.

`org join URL ORG --fingerprint F` asks to join. Sign in to the server first. pasejo enrolls the device, fetches the membership log and compares the fingerprint of its first entry with `F`; without `--fingerprint`, it asks interactively. When they match, the organization is registered locally and every later replay starts from that entry. A device whose fingerprint check fails is never registered, so its enrollment stays unused. Running `org join` again after approval reports that the device is active.

## Devices and members

`org devices` lists active devices and pending enrollments with their fingerprints.

`org role EMAIL ROLE` gives a person a role (`admin`, `approver`, `auditor` or `member`), or removes them with `none`. A person waiting for approval can be found by email address already, so the usual order is `org role bob@example.com member` followed by `org approve`.

`org approve DEVICE --fingerprint F` admits a pending device. Compare the fingerprint its owner sees with `F` before approving; the device's owner must already be a member. `org revoke DEVICE` removes a device. A revoked device never returns; enrolling again creates a new one.

`org team create NAME`, `org team add TEAM EMAIL` and `org team remove TEAM EMAIL` manage teams. Grants to a team (see `store grant`) apply to all its members.

## Settings and second admins

`org settings` changes `--device-approval` (`manual` or `automatic`), `--two-admin` and `--auto-join`. With `--two-admin true`, role, setting and recovery changes need a second admin's signature. Turning it on also needs one. Such a change is recorded as a proposal, and pasejo prints its identifier. Another admin lists proposals with `org proposals` and signs one with `org cosign PROPOSAL`.

## Pending actions

`org actions` lists the work the log asks devices to do: approving devices automatically, adding people who signed in with `auto_join`, wrapping store keys for newly entitled devices, rotating store keys after someone lost access, and co-signing audit checkpoints. `org actions --run` performs every action this device may perform. `store grant` runs the key wrapping and rotation of its store itself.

## Recovery

`org recovery set RECIPIENT` sets the age recipient whose identity the organization keeps offline. After the next `org actions --run`, every store's keys are also wrapped for it.

`org recover --identity FILE --output DIR` is for admins. It downloads every store and decrypts it with the recovery identity. It writes one file store per remote store to `DIR/<store>.age`, encrypted to the recovery recipient, so the data stays usable without the server: register a file with `pasejo store add` and the recovery identity with `pasejo identity add`.

## Stores

`org stores` lists the organization's stores this device sees, with the label of each store whose keys the device holds, your role and the local name under which it is registered.
