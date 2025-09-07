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
- Execute automated commands before/after reading/writing secrets/passwords 

> [!IMPORTANT]
> `pasejo` relies on the [age crate](https://crates.io/crates/age) which itself is in beta and not intended for production use. Therefore, this project is not intended for production use either. Use at your own risk.

![usage](./vhs/demo.gif)

## Usage

Checkout the [docs](./docs/README.md) if you want to learn more about `pasejo` and how to use it.
