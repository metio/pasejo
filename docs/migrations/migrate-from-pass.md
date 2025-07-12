# Migrate from pass to pasejo

You can migrate from `pass` to `pasejo` using the following snippet. It iterates over all secrets in `pass` and adds them to a `pasejo` store.

```shell
# add all secrets from pass to pasejo
$ find "${PASSWORD_STORE_DIR}" -name '*.gpg' -type f -print | \
    sed s,"${PASSWORD_STORE_DIR}/",, | \
    sed s,.gpg,, | \
    xargs -I {} sh -c 'pass {} | head --bytes -1 | pasejo secret add {} --offline'

# sync local changes to the remote store
$ pasejo store sync --push
```

This snippet assumes that you have already set up a `pasejo` store and that you have the `PASSWORD_STORE_DIR` environment variable set to the path of your `pass` store. Additionally, you need to add identities and recipients to your `pasejo` store before running this command to allow for encryption/decryption to work properly.
