# Add a store

Adding a store is the first step to using Pasejo. A store is a collection of secrets, one-time passwords, and recipients that can be shared with other users. You can add a store using the [pasejo store add](../commands/pasejo-cmd-store.md) command.

## Add a store

You can add a store like this:

```shell
$ pasejo store add --path /path/to/store --name store-name
```

This will register a new store at the specified path with the given name. The path will be created once you have added the first recipient to the store.

## Adding a default store

You can also add a store as the default store. This will make it the default store for all Pasejo commands that require a store. To do this, use the `--default` flag:

```shell
$ pasejo store add --path /path/to/store --name store-name --default
```

This will register a new store at the specified path with the given name and set it as the default store. You won't have to specify the store name in subsequent commands, as `pasejo` will use the default store automatically.
