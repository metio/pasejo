# Create age recipients

age recipients are used to encrypt stores in `pasejo`. This guide will help you create age recipient files that can be used with your `pasejo` configuration.

## Native age recipients

You can create age recipient files using the `age-keygen` command line tool. For example, to create a new recipient file, run:

```shell
# create identity first
$ age-keygen --output ~/.local/share/pasejo/identities/my-identity.age

# create recipient for the newly created identity
$ age-keygen -y > ~/.local/share/pasejo/recipients/my-recipient.age < ~/.local/share/pasejo/identities/my-identity.age
```

This will create a new recipient file at the specified path. You can add this file to your store using the `pasejo recipient add` command and use it to encrypt stores. Refer to the [age-keygen documentation](https://filippo.io/age/age-keygen.1) for more details on how to use it.

## SSH keys as age recipients

You can also use SSH keys as age recipients. Use the `ssh-keygen` command to create a new SSH key pair, which can then be used as an age recipient. For example, to create a new SSH key pair, run:

```shell
# create public/private key pair
$ ssh-keygen -t ed25519 -f ~/.local/share/pasejo/identities/my-identity.ssh

# copy .pub file to the recipients folder
$ cp ~/.local/share/pasejo/identities/my-identity.ssh.pub ~/.local/share/pasejo/recipients/my-recipient.ssh
```

The `ssh-keygen` command will create both a private key (identity) and public key (recipient) at the specified path. You can then add the public key file to your store using the `pasejo recipient add` command and use it to encrypt stores. Refer to the [ssh-keygen documentation](https://man.openbsd.org/ssh-keygen) for more details on how to use it.
