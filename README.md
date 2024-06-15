<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
 -->

# pasejo

A simple, secure, and easy-to-use password manager for teams. It is designed to be a self-hosted solution that allows you to store and share passwords securely. It uses [age](https://age-encryption.org/) keys for encryption and decryption, and it is designed to be used with a command-line interface (CLI).

**NOTE**: This relies on the [age crate](https://crates.io/crates/age) which itself is in beta and not intended for production use. Therefore, this password manager is not intended for production use either. Use at your own risk.

## Usage

Add a new store to your configuration first:

```console
$ pasejo stores add --path path/to/store --name some-name
```

Add an age identity file to your store:

```console
$ pasejo identity add --file path/to/age/private/key --store some-name
```

Add an age recipient file to your store:

```console
$ pasejo recipient add --file path/to/age/public/key --store some-name
```

Add a new secret to your store:

```console
$ pasejo secret add some-secret --store another
```

See `pasejo help` for more details.

## How-to guides

### Create age identity files

You can create age identity files using the `age-keygen` command line tool. For example, to create a new identity file, run:

```console
$ age-keygen --output path/to/age/private/key
```

This will create a new identity file at the specified path. You can then use this file to decrypt secrets.

### Create age recipient files

You can create age recipient files using the `age-keygen` command line tool. For example, to create a new recipient file, run:

```console
$ age-keygen -y > path/to/age/public/key < path/to/age/private/key 
```
This will create a new recipient file at the specified path. You can then use this file to encrypt secrets.

### Use SSH keys

The age crate supports SSH keys as well. You can re-use existing SSH keys to create age identity and recipient files. For example, to add an SSH key as an identity file, run:

```console
$ pasejo identity add --file ~/.ssh/id_rsa --store some-name
```

To add an SSH key as a recipient file, run:

```console
$ pasejo recipient add --file ~/.ssh/id_rsa.pub --store some-name
```
