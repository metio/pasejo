```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo secret generate some-secret
Generated secret at 'some-secret'

$ pasejo secret generate another-secret
Generated secret at 'another-secret'

$ pasejo secret generate nested/secret
Generated secret at 'nested/secret'

$ pasejo secret generate deeply/nested/secret
Generated secret at 'deeply/nested/secret'

$ pasejo export bitwarden --pretty
Warning: the export contains every secret in plaintext.
{
  "folders": [
    {
      "id": "[..]",
      "name": "deeply/nested"
    },
    {
      "id": "[..]",
      "name": "nested"
    }
  ],
  "items": [
    {
      "id": "[..]",
      "folderId": null,
      "organizationId": null,
      "collectionIds": [],
      "name": "another-secret",
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
      "folderId": "[..]",
      "organizationId": null,
      "collectionIds": [],
      "name": "deeply/nested/secret",
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
      "folderId": "[..]",
      "organizationId": null,
      "collectionIds": [],
      "name": "nested/secret",
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
      "name": "some-secret",
      "notes": "",
      "fields": [],
      "login": {
        "username": "",
        "password": "[..]",
        "uris": []
      },
      "type": 1
    }
  ]
}

```
