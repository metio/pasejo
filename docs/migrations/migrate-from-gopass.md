# Migrate from gopass to pasejo

You can migrate from `gopass` to `pasejo` using the following snippet. It iterates over all secrets in `gopass` and adds them to a `pasejo` store.

```shell
# add all secrets from gopass to pasejo
$ find "${PASSWORD_STORE_DIR}" -name '*.gpg' -type f -print | \
    sed s,"${PASSWORD_STORE_DIR}/",, | \
    sed s,.gpg,, | \
    xargs -I {} sh -c 'gopass show {} | head --bytes -1 | pasejo secret add {} --offline'

# sync local changes to the remote store
$ pasejo hook run --push
```

This snippet assumes that you have already set up a `pasejo` store and that you have the `PASSWORD_STORE_DIR` environment variable set to the path of your `gopass` store. Additionally, you need to add identities and recipients to your `pasejo` store before running this command to allow for encryption/decryption to work properly.
