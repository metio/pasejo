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

## Policies

`age-plugin-yubikey --generate` accepts `--touch-policy` (`always`, `cached` or `never`, default `always`) and `--pin-policy` (`always`, `once` or `never`, default `once`). With the touch policy `always` the YubiKey needs a touch for every decryption; with `cached` it needs one only when more than 15 seconds have passed since it was last touched. With the PIN policy `once` the plugin keeps the YubiKey's PIN cache between decryptions, so the PIN is asked for once until the YubiKey is unplugged; according to the plugin's documentation this does not work on the YubiKey 4 series. The policies are fixed when the identity is generated.

## Browser use

A YubiKey identity is the way to have the browser extension release each credential only after you confirm it on the key. The native-messaging host behind the extension holds no unlocked store between requests: it decrypts the store for every query and again for every fill, and each decryption runs the plugin once. With the touch policy `always`, opening the extension's popup therefore asks for one touch for each YubiKey-backed store it searches, and filling a credential asks for one more, so no password reaches a page without a touch for that password. This holds as long as no other identity configured for the store can decrypt it, since an identity file on disk needs no touch. With the touch policy `cached`, a fill within 15 seconds of the last touch needs no second one, because the YubiKey itself remembers the touch for that long.

The host has no terminal, so a PIN request from the plugin, and its request to insert a YubiKey that is not plugged in, are shown through pinentry: the program named by `PINENTRY_PROGRAM`, or `pinentry` on the `PATH`. Browsers start the host with a reduced environment, so pin a graphical pinentry when installing the manifest:

```console
$ pasejo browser install-manifest --browser firefox --env PINENTRY_PROGRAM=/usr/bin/pinentry-gnome3
```

A pinentry dialog closes itself after 30 seconds. When no pinentry can be found, the host declines the plugin's request instead of asking on a terminal and answers the extension with `identity_locked`. With `ignore-missing-identities` enabled (the default), pasejo leaves out YubiKey identities whose YubiKey is not connected, so a store that can also be decrypted with another identity does not wait for an absent key.

While one store waits for a touch or a PIN, the host keeps answering requests for other stores, and the store itself stays unlocked, so `pasejo` commands in a terminal are not held up. Messages the plugin prints go to the host's standard error, which browsers keep in their logs rather than in the popup. See [`pasejo browser`](../commands/pasejo-cmd-browser.md) for setting up the host.
