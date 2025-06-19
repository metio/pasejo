# Using pasejo with Git

`pasejo` has rudimentary support for remote synchronization using Git. This allows you to keep your secrets in sync across multiple devices or share them with team members.

## Setting Up a Git Store

To use `pasejo` with Git, you first need to create a Git repository that will serve as your store. You can do this by running the following command:

```shell
$ git init /path/to/your/repo
```

This command initializes a new Git repository at the specified path. Make sure to replace `/path/to/your/repo` with the actual path where you want to create the repository. Once the repository is created, you can add a `pasejo` store into it and enable synchronization:

```
$ pasejo store add --path /path/to/your/repo/pasejo.store --name my-git-store --synchronizer git
```

This command will create a new store named `my-git-store` at the specified path. Make sure that the path points inside a valid Git repository.

## Using the Git Store

Once initialized, you can use the `pasejo` commands to manage your secrets as usual. For example, you can add secrets, show them, and so on:

```shell
$ pasejo secret add some-secret
$ pasejo secret show some-secret
```

When you make changes to your secrets, `pasejo` will automatically track these changes in the Git repository and push changes to the remote repository if configured.
