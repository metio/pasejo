```
$ pasejo recipient list
? 1
Error: Der Speicher 'team' ist ein entfernter Speicher; Empfänger, Hooks und Zusammenführungen gelten für ihn nicht.

$ pasejo hook get
? 1
Error: Der Speicher 'team' ist ein entfernter Speicher; Empfänger, Hooks und Zusammenführungen gelten für ihn nicht.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' ist kein Datum im Format JJJJ-MM-TT.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: Der Speicher 'files' ist kein entfernter Speicher.

$ pasejo org devices --org nope
? 1
Error: Keine registrierte Organisation passt zu 'nope'.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: Unter https://other.example ist kein Server registriert. Führen Sie 'pasejo server login https://other.example' aus.

$ pasejo org devices
? 1
Error: Dieses Gerät ist nicht bei https://pasejo.example angemeldet. Führen Sie 'pasejo server login https://pasejo.example' aus.

$ pasejo server login https://other.example --emergency
? 1
Error: Die Notfall-Anmeldung braucht ein Gerät, das bereits bei https://other.example registriert ist.

```
