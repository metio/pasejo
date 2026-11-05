A TOTP entry with the same name as a secret is exported as that login's
`totp` value in `otpauth://` form. Bitwarden cannot hold HOTP entries, and
an OTP entry without a same-named secret has no login to attach to; both
are left out and named in a warning on stderr.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate github
Generated secret at 'github'

$ pasejo secret generate bank
Generated secret at 'bank'

$ pasejo otp add --secret JEQDYMZAN5YGK3RAONXXK4TDMU github
Added one-time password at 'github'

$ pasejo otp add --type hotp --counter 5 --secret JEQDYMZAN5YGK3RAONXXK4TDMU bank
Added one-time password at 'bank'

$ pasejo otp add --secret JEQDYMZAN5YGK3RAONXXK4TDMU orphan
Added one-time password at 'orphan'

$ pasejo export bitwarden --pretty
Warning: the export contains every secret in plaintext.
Warning: Bitwarden cannot store HOTP one-time passwords, so these are not part of the export: bank
Warning: these one-time passwords have no secret of the same name and are not part of the export: orphan
{
  "folders": [],
  "items": [
    {
      "id": "[..]",
      "folderId": null,
      "organizationId": null,
      "collectionIds": [],
      "name": "bank",
      "notes": "",
      "fields": [],
      "login": {
        "username": "",
        "password": "[..]",
        "uris": []
      },
      "type": 1
    },
    {
      "id": "[..]",
      "folderId": null,
      "organizationId": null,
      "collectionIds": [],
      "name": "github",
      "notes": "",
      "fields": [],
      "login": {
        "username": "",
        "password": "[..]",
        "uris": [],
        "totp": "otpauth://totp/github?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&algorithm=SHA1&digits=6&period=30"
      },
      "type": 1
    }
  ]
}

```
