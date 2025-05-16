<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
 -->

# pasejo

`pasejo` (esperanto for `passage`) is a re-implementation of [passage](https://github.com/FiloSottile/passage). It uses [age](https://age-encryption.org/) keys for encryption and decryption, stores all your passwords locally, and offers a simple command line interface.

It saves all your secrets in one single encrypted file, and has built-in support to synchronize with a remote repository using [Git](https://git-scm.com/), [Mercurial](https://mercurial-scm.org/), or [Pijul](https://pijul.org/).

**NOTE**: `pasejo` relies on the [age crate](https://crates.io/crates/age) which itself is in beta and not intended for production use. Therefore, this project is not intended for production use either. Use at your own risk.

## Installation

You can install `pasejo` using `cargo`:

```console
$ cargo install pasejo
```

Alternatively, you can download the latest version from the [release page](https://github.com/metio/pasejo/releases/latest) and extract the archive. The binary is named `pasejo` and can be run directly from the extracted folder.

## Concepts

### Store

A store is an encrypted file that contains the secrets and recipients. You can create multiple stores to organize your secrets. Each store is encrypted with the keys of the registered recipients. You can decrypt stores with a matching identity file.

### Identity

An identity is a private key used to decrypt stores. You can add multiple identities for different stores. Identities can be used for multiple stores as well as registered globally for all stores.

### Recipient

A recipient is a public key used to encrypt stores. You can add multiple recipients for different stores.

### Secret

A secret is an arbitrary text stored in a store. You can add, remove, and list secrets in a store. Secrets are encrypted with the keys of the registered recipients. You can decrypt secrets with a matching identity file.

## Usage

Add a new store to your configuration first:

```console
$ pasejo stores add --path path/to/store --name some-name
```

Add an age identity file to your store:

```console
$ pasejo identity add --file path/to/age/private/key
```

Add an age recipient file to your store:

```console
$ pasejo recipient add --file path/to/age/public/key
```

Add a new secret to your store:

```console
$ pasejo secret add some-secret
```

Show a secret in your store:

```console
$ pasejo secret show some-secret
```

See `pasejo help` for more details.

## How-to guides

### Create age identity files

You can create age identity files using the `age-keygen` command line tool. For example, to create a new identity file, run:

```console
$ age-keygen --output path/to/age/private/key
```

This will create a new identity file at the specified path. You can then use this file to decrypt stores.

### Create age recipient files

You can create age recipient files using the `age-keygen` command line tool. For example, to create a new recipient file, run:

```console
$ age-keygen -y > path/to/age/public/key < path/to/age/private/key 
```
This will create a new recipient file at the specified path. You can then use this file to encrypt stores.

### Use SSH keys

The age crate supports SSH keys as well. You can re-use existing SSH keys as identity and recipient files. For example, to add an SSH key as an identity file, run:

```console
$ pasejo identity add --file ~/.ssh/id_rsa
```

To add an SSH key as a recipient file, run:

```console
$ pasejo recipient add --file ~/.ssh/id_rsa.pub
```

## Alternatives

In case you are looking for something different, try these:

- [passage](https://github.com/FiloSottile/passage)
- [pass](https://www.passwordstore.org/)
- [pago](https://github.com/dbohdan/pago)
- [pa](https://github.com/biox/pa)
- [kbs2](https://github.com/woodruffw/kbs2)
- [seniorpw](https://gitlab.com/retirement-home/seniorpw)
- [trespas](https://gitlab.com/pizkaz/trespass)
- [gopass](https://www.gopass.pw/)
