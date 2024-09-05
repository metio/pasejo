# pasejo

passage re-implementation in Rust for teams.

## Usage

Initialize a new store first:

```console
$ pasejo stores init --path path/to/store --alias some-name --vcs git
```

## TODO

`infisical run` like interface:

```shell
$ pasejo run --env <name> -- <command> <args>
```

envs are defined in a `pasejo.toml` file in the root of a project

```toml
default_env = "dev"

[envs.dev]
SOME_KEY_HERE = "path/to/secret"
ANOTHER_KEY = "path/to/other"
```
