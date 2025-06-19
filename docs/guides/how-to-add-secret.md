# Add secrets

Secrets are part of a store and shared with all users of the store. They can be arbitrary long text in any format and typically contain the 'secret' or 'password' in their first line. You can add a secret to a specific store using the [pasejo secret add](../commands/pasejo-cmd-secret.md) command.

## Add a single-line secret

You can add a single-line secret to your store like this:

```shell
$ pasejo secret add some-secret
```

`pasejo` will prompt you to enter the secret, and it will be stored in the store with the name `some-secret`.

## Add a multi-line secret

You can add a multi-line secret to your store like this:

```shell
$ pasejo secret add some-secret --multiline
```

`pasejo` will open your configured editor, and you can enter the secret in the editor. When you save and close the editor, the secret will be stored in the store with the name `some-secret`.

## Add secret from stdin

You can also add a secret from stdin like this

```shell
$ echo "my secret" | pasejo secret add some-secret
```

This will read the secret from stdin and store it in the store with the name `some-secret`.

## Generating a secret

You can also generate a secret using the `--generate` flag. For example, to generate a random password, you can run:

```shell
$ pasejo secret generate --length 42 some-secret
```

This will generate a random password with a length of 42 characters and store it in the store with the name `some-secret`.
