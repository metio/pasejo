# Concepts

The main concepts of `pasejo` are described below. They are used throughout the documentation as well as the user interface.

## Store

A store is an encrypted file that contains the secrets and recipients. You can create multiple stores to organize your secrets. Each store is encrypted with the keys of the registered recipients. You can decrypt stores with an identity matching one of the registered recipients.

Related commands:

- [pasejo store](../commands/pasejo-cmd-store.md)

## Identity

An identity is a private key used to decrypt stores. You can add multiple identities to the same store, e.g., for backup keys, and register identities globally for all your stores. Identities are local to your system and are not shared with other users of the same store.

Related commands:

- [pasejo identity](../commands/pasejo-cmd-identity.md)

## Recipient

A recipient is a public key used to encrypt stores. You can add multiple recipients to the same store, allowing multiple users to interact with the store. Recipients are part of a store and shared with all users of the store. If you need a different set of recipients for some of your secrets, you must create a new store and add the appropriate recipients to it.

Related commands:

- [pasejo recipient](../commands/pasejo-cmd-recipient.md)

## Secret

A secret is an arbitrary text stored in a store. Typically, the first line is something like a password, whereas the following lines include additional metadata or information about the secret. For example, the following secret could be used to store a password for a service:

```
my-secret-password
username: my-user
url: https://example.com
```

Related commands:

- [pasejo secret](../commands/pasejo-cmd-secret.md)

## One-time password

A one-time password (OTP) is a secret that can be used only once. You can add OTPs to a store and use them to authenticate with services that support OTPs. `pasjo` supports both TOPT and HOTP algorithms for generating one-time passwords.

Related commands:

- [pasejo otp](../commands/pasejo-cmd-otp.md)

## Configuration

`pasejo` has a user-specific configuration file. Values saved in the configuration only apply to your local system and are not shared with everyone else using the same store.

Related commands:

- [pasejo config](../commands/pasejo-cmd-config.md)
