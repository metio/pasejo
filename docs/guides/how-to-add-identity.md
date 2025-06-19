# Add Identities

You can add identities to a single store or make an identity available to all stores in your configuration. You can add an identity using the [pasejo identity add](../commands/pasejo-cmd-identity.md) command

## Add an identity to a store

Add an identity to a single store like this:

```shell
$ pasejo identity add --file path/to/age/private/key --store some-store
```

This will add the specified identity file to the store. You can then use this identity to decrypt secrets in that store. No other store will be able to use this identity (as long as you don't add the same identity to those other stores).

## Add an identity to all stores

You can also add an identity to all stores in your configuration using the `pasejo identity add` command with the `--global` flag. For example, to add an identity file globally, run:

```shell
$ pasejo identity add --file path/to/age/private/key --global
```

This will add the specified identity file to all stores in your configuration. You can then use this identity to decrypt secrets in any store that has access to it.
