```
$ pasejo recipient list
? 1
Error: The store 'team' is a remote store; recipients, hooks and merges do not apply to it.

$ pasejo hook get
? 1
Error: The store 'team' is a remote store; recipients, hooks and merges do not apply to it.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' is not a date in the form YYYY-MM-DD.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: The store 'files' is not a remote store.

$ pasejo org devices --org nope
? 1
Error: No registered organization matches 'nope'.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: No server is registered at https://other.example. Run 'pasejo server login https://other.example'.

$ pasejo org devices
? 1
Error: This device is not signed in to https://pasejo.example. Run 'pasejo server login https://pasejo.example'.

```
