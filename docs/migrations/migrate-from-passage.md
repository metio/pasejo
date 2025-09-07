# Migration from passage to pasejo

You can migrate from `passage` to `pasejo` using the following snippet. It iterates over all secrets in `passage` and adds them to a `pasejo` store.

```shell
# ensure your local copy has the same state as your remote store
$ pasejo hook run --pull

# add all recipients from passage to pasejo
$ pasejo recipient add --file "${PASSAGE_DIR}/.age-recipients"

# add all secrets from passage to pasejo
$ find "${PASSAGE_DIR}" -name '*.age' -type f -print | \
    sed s,"${PASSAGE_DIR}/",, | \
    sed s,.age,, | \
    xargs -I {} sh -c 'passage show {} | head --bytes -1 | pasejo secret add {} --offline'

# sync local changes to the remote store
$ pasejo hook run --push
```
