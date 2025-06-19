# Using Yubikeys with pasejo

You can store age identities on your Yubikeys using [age-plugin-yubikey](https://github.com/str4d/age-plugin-yubikey). Follow its [instructions](https://github.com/str4d/age-plugin-yubikey?tab=readme-ov-file#configuration) to create a new identity, export it to a file, and add it to your store like this:

```console
# generate a new identity
$ age-plugin-yubikey --generate --slot SLOT

# export the identity to a file
$ age-plugin-yubikey --identity --slot SLOT > yubikey-identity.txt

# add the identity to your store
$ pasejo identity add --file yubikey-identity.txt

# list all recipients on your Yubikey
$ age-plugin-yubikey --list

# add recipient to your store
$ pasejo recipient add --public-key RECIPIENT-FROM-YUBIKEY
```

We highly recommend using at least two Yubikeys for redundancy. You can add the identities and recipients from both Yubikeys to your store. If you lose one, you can still access your secrets with the other one. `pasejo` will automatically use the first available identity when decrypting a store.
