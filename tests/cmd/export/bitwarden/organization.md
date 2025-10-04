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

$ pasejo export bitwarden --organization-id xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx --pretty
{
  "collections": [
    {
      "id": "[..]",
      "name": "pasejo",
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "external_id": null
    }
  ],
  "items": [
    {
      "id": "[..]",
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "[..]"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "[..]"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "[..]"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "[..]"
      ],
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

$ pasejo export bitwarden --organization-id xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx --collection-id yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy --pretty
{
  "collections": [
    {
      "id": "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy",
      "name": "pasejo",
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "external_id": null
    }
  ],
  "items": [
    {
      "id": "[..]",
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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

$ pasejo export bitwarden --organization-id xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx --collection-id yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy --collection-name some-collection --pretty
{
  "collections": [
    {
      "id": "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy",
      "name": "some-collection",
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "external_id": null
    }
  ],
  "items": [
    {
      "id": "[..]",
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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
      "folder_id": null,
      "organization_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "collection_ids": [
        "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
      ],
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

$ pasejo export bitwarden --collection-id yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy --collection-name some-collection
? 2
error: the following required arguments were not provided:
  --organization-id <ORGANIZATION_ID>

Usage: pasejo export bitwarden --organization-id <ORGANIZATION_ID> --collection-id <COLLECTION_ID> --collection-name <COLLECTION_NAME>

For more information, try '--help'.

$ pasejo export bitwarden --collection-id yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy
? 2
error: the following required arguments were not provided:
  --organization-id <ORGANIZATION_ID>

Usage: pasejo export bitwarden --organization-id <ORGANIZATION_ID> --collection-id <COLLECTION_ID>

For more information, try '--help'.

$ pasejo export bitwarden --collection-name some-collection
? 2
error: the following required arguments were not provided:
  --organization-id <ORGANIZATION_ID>

Usage: pasejo export bitwarden --organization-id <ORGANIZATION_ID> --collection-name <COLLECTION_NAME>

For more information, try '--help'.

```
