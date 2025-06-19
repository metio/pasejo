# Add recipients

Recipients are part of a store and shared with all users of the store. Therefore, you can only add recipients to a store and not globally like identities. You can add a recipient to a specific store using the [pasejo recipient add](../commands/pasejo-cmd-recipient.md) command.

## Add a recipient from a file

You can add recipients saved in a file to your store like this:

```shell
$ pasejo recipient add --file path/to/age/public/key
```

If the file contains multiple recipients, they will all be added to the store.

## Add a recipient from Codeberg

Codeberg exposes the public key of their users, and you can add their key as a recipient to your store. For example, to add the public key of a user with the username `codeberg-user`, run:

```shell
$ pasejo recipient add --codeberg codeberg-user
```

## Add a recipient from GitHub

GitHub exposes the public key of their users, and you can add their key as a recipient to your store. For example, to add the public key of a user with the username `github-user`, run:

```shell
$ pasejo recipient add --github github-user
```

## Add a recipient from GitLab

GitLab exposes the public key of their users, and you can add their key as a recipient to your store. For example, to add the public key of a user with the username `gitlab-user`, run:

```shell
$ pasejo recipient add --gitlab gitlab-user
```
