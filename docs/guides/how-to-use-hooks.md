# Using hooks

`pasejo` exposes hooks to allow users running arbitrary commands during the execution of `pasejo` commands. The available hooks are:

- `pull`
- `push`

In general, the `pull` hook is executed before any data is read from a store. The `push` hook is executed after something was written to a store.

You can configure hook commands both per store and globally for all stores, e.g., if you want all your stores to be synchronized to a remote Git repository, you can define global hook commands that do just that.

The execution order of global and per-store commands is as follows:

1. global pull commands
2. store pull commands
3. global push commands
4. store push commands

You can use the following placeholders in your command. They will be replaced with the actual value before executing your command:

- `%p`: The absolute path to the store
