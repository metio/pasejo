# Create age identities

age identities are used to decrypt stores in `pasejo`. This guide will help you create age identity files that can be used with your `pasejo` configuration.

## Native age identities

You can create age identity files using the `age-keygen` command line tool. For example, to create a new identity file, run:

```shell
$ age-keygen --output ~/.local/share/pasejo/identities/my-identity.age
```

This will create a new identity file at the specified path. You can add this file to your store using the `pasejo identity add` command and use it to decrypt stores. Refer to the [age-keygen documentation](https://filippo.io/age/age-keygen.1) for more details on how to use it.

## SSH keys as age identities

You can also use SSH keys as age identities. Use the `ssh-keygen` command to create a new SSH key pair, which can then be used as an age identity. For example, to create a new SSH key pair, run:

```shell
$ ssh-keygen -t ed25519 -f ~/.local/share/pasejo/identities/my-identity.ssh
```

This will create a new identity file at the specified path. You can then add this file to your store using the `pasejo identity add` command and use it to decrypt stores. Refer to the [ssh-keygen documentation](https://man.openbsd.org/ssh-keygen) for more details on how to use it.
