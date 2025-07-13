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

This command will register a new store named `my-git-store` at the specified path. Make sure that the path points inside a valid Git repository. The filename `pasejo.store` can be any name you choose, and it's safe to pick any path within your Git repository.

## Using the Git Store

Once initialized, you can use the `pasejo` commands to manage your secrets as usual. For example, you can add secrets, show them, and so on:

```shell
$ pasejo secret add some-secret
$ pasejo secret show some-secret
```

When you make changes to your secrets, `pasejo` will automatically track these changes in the Git repository and push changes to the remote repository if configured. Likewise, it will pull changes from the remote repository before running any command that would interact with a store, like `pasejo secret show ...`.

## Configuring Git Diff

To configure `git diff` to show the differences between two versions of a store, you can use the following commands:

```shell
# use the same file extension as the store file you registered previously
$ echo '*.store diff=pasejo' >> /path/to/your/repo/.gitattributes

# declare that pasejo files are binary files
$ git -C /path/to/your/repo config --local diff.pasejo.binary true

# configure the text conversion command for pasejo files
$ git -C /path/to/your/repo config --local diff.pasejo.textconv "pasejo store decrypt --store my-git-store --store-path"
```

## Configuring Git Merge

To configure `git merge` to handle conflicts in `pasejo` stores, you can set up a custom merge driver. This allows Git to use `pasejo` to resolve conflicts when merging changes from different branches.

```shell
# use the same file extension as the store file you registered previously
$ echo '*.store merge=pasejo' >> /path/to/your/repo/.gitattributes

# declare that pasejo files are binary files
$ git -C /path/to/your/repo config --local merge.pasejo.driver "pasejo store merge --store my-git-store --common-ancestor %O --current-version %A --other-version %B"
```
