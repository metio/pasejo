<!--
SPDX-FileCopyrightText: The pasejo Authors
SPDX-License-Identifier: 0BSD
 -->

# pasejo

`pasejo` (esperanto for `passage`) is a re-implementation of [passage](https://github.com/FiloSottile/passage) in Rust for teams. It uses [age](https://age-encryption.org/) keys for encryption and decryption, stores all your passwords locally, and offers a simple command line interface.

Its main features are:

- Support multiple stores with multiple recipients and identities
- Support for SSH and age keys as identities and recipients
- Support for text-based secrets as well as one-time passwords (TOTP/HOTP)
- Clipboard support for secrets and one-time passwords
- Completions for various shells (bash, elvish, fish, powershell, zsh)
- Rudimentary support for remote sync using Git, Mercurial, or Pijul

**NOTE**: `pasejo` relies on the [age crate](https://crates.io/crates/age) which itself is in beta and not intended for production use. Therefore, this project is not intended for production use either. Use at your own risk.

![usage](./vhs/demo.gif)

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
$ pasejo store add --path path/to/store --name some-name
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

### Using Yubikeys

You can store age identities on your Yubikeys using [age-plugin-yubikey](https://github.com/str4d/age-plugin-yubikey). Follow its [instructions](https://github.com/str4d/age-plugin-yubikey?tab=readme-ov-file#configuration) to create a new identity, export it to a file, and add it to your store like this:

```console
# generate a new identity
$ age-plugin-yubikey --generate --slot SLOT

# export the identity to a file
$ age-plugin-yubikey --identity --slot SLOT > yubikey-identity.txt

# add the identity to your store
$ pasejo identity add --file yubikey-identity.txt

# list all recipients on your Yubikey
$ age-plugin-yubikey --list

# add recipient to your store
$ pasejo recipient add --public-key RECIPIENT-FROM-YUBIKEY
```

We highly recommend using at least two Yubikeys for redundancy. You can add the identities and recipients from both Yubikeys to your store. If you lose one, you can still access your secrets with the other one. `pasejo` will automatically use the first available identity when decrypting a store.

### Create shell completions

You can create shell completions for your shell of choice using the `COMPLETE` environment variable. For example, to create a bash completion file, run:

```console
$ COMPLETE=bash pasejo > ${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/pasejo
```

The supported shells are:
- `bash`
- `elvish`
- `fish`
- `powershell`
- `zsh`

Check their respective documentation on where to install completion files.

### Migrate from passage

You can migrate from `passage` to `pasejo` using the following snippet. It iterates over all secrets in passage and adds them to a pasejo store.

```console
$ pasejo store sync --pull
$ find "${PASSAGE_DIR}"  -name '*.age' -type f -print | \
    sed s,"${PASSAGE_DIR}/",, | \
    sed s,.age,, | \
    xargs -I {} sh -c 'passage show {} | pasejo secret add {} --offline'
$ pasejo store sync --push
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
- [psswd](https://github.com/Gogopex/psswd)
- [passpie](https://github.com/marcwebbie/passpie)
- [privage](https://github.com/revelaction/privage)
- [pwd.sh](https://github.com/drduh/pwd.sh)
