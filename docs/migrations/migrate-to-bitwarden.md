# Migration from pasejo to bitwarden

You can migrate from `pasejo` to `bitwarden` using the built-in `export` command like this:

```shell
# export in individual format
$ pasejo export bitwarden

# export in organization format
$ pasejo export bitwarden --organization-id some-id-here

# export into existing organization collection
$ pasejo export bitwarden --organization-id existing-organization-id --collection-id existing-collection-id --collection-name existing-collection-name
```

> **Note:** the exported JSON contains every secret in **cleartext**.
> Redirecting it to a file (`> backup.json`) creates a world-readable
> file under your default umask and leaves the command in your shell
> history. Prefer `--out <FILE>`, which writes the file with `0600`
> permissions, and delete it as soon as the import is done.

A TOTP entry is exported as the `totp` value of the login with the same
name. HOTP entries and OTP entries without a same-named secret are not
part of the export; the command lists them on stderr. Rename an OTP entry
with `pasejo otp move` to pair it with its secret before exporting, and
re-enroll HOTP entries in an authenticator that supports them.
