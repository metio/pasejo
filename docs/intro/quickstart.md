# Quickstart Guide

This guide will help you get started with `pasejo`. Refer to the remaining documentation for more detailed information. We assume that `pasejo` is already installed and available in your `PATH`. Check out the [installation guide](./installation.md) if you haven't done so yet. Likewise, we assume that you already have an [age](https://filippo.io/age/age-keygen.1) or [SSH](https://man.openbsd.org/ssh-keygen) key generated. If you don't have it yet, please refer their respective upstream documentation.

```shell
# create a folder for your store
$ mkdir --parents ~/.local/share/pasejo/stores

# add a new store
$ pasejo store add --path ~/.local/share/pasejo/stores/my-store --name "My Store"

# add an identity to your store
$ pasejo identity add --file ~/.local/share/pasejo/identities/my-identity.age

# add a recipient to your store
$ pasejo recipient add --file ~/.local/share/pasejo/recipients/my-recipient.age

# add a new secret to your store
$ pasejo secret add my-secret

# show the secret in your store
$ pasejo secret show my-secret
```

Detailed information can be found here:

- [Add a new store](../guides/how-to-add-store.md): Learn how to add a new store.
- [Create age identity files](../guides/how-to-create-age-identity.md): Learn how to create age identity files.
- [Create age recipient files](../guides/how-to-create-age-recipient.md): Learn how to create age recipient files.
- [Add an age identity file](../guides/how-to-add-identity.md): Learn how to add an age identity file to your store.
- [Add an age recipient file](../guides/how-to-add-recipient.md): Learn how to add an age recipient file to your store.
- [Add a new secret](../guides/how-to-add-secret.md): Learn how to add a new secret to your store.
- [Add a new one-time password](../guides/how-to-add-otp.md): Learn how to add a new one-time password to your store.
- [Show a secret](../guides/how-to-show-secret.md): Learn how to show a secret in your store.
