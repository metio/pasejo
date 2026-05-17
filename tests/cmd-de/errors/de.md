```
$ pasejo secret add some-secret
? 1
Error: Kein Speicher in der Konfiguration gefunden. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen

$ pasejo store add --path store --name primary
Speicher 'primary' bei '[CWD]/store' hinzugefügt

$ pasejo store add --path other --name primary
? 1
Error: Speichername existiert bereits. Bitte verwenden Sie einen anderen Namen.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': Speicher mit Name 'missing' existiert nicht in der Konfiguration

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
