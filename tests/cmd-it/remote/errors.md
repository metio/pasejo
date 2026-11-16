```
$ pasejo recipient list
? 1
Error: L'archivio 'team' è remoto; destinatari, hook e unioni non si applicano.

$ pasejo hook get
? 1
Error: L'archivio 'team' è remoto; destinatari, hook e unioni non si applicano.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' non è una data nel formato AAAA-MM-GG.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: L'archivio 'files' non è un archivio remoto.

$ pasejo org devices --org nope
? 1
Error: Nessuna organizzazione registrata corrisponde a 'nope'.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: Nessun server è registrato su https://other.example. Esegui 'pasejo server login https://other.example'.

$ pasejo org devices
? 1
Error: Questo dispositivo non ha effettuato l'accesso a https://pasejo.example. Esegui 'pasejo server login https://pasejo.example'.

```
