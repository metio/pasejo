```
$ pasejo recipient list
? 1
Error: El almacén 'team' es remoto; los destinatarios, hooks y fusiones no se aplican a él.

$ pasejo hook get
? 1
Error: El almacén 'team' es remoto; los destinatarios, hooks y fusiones no se aplican a él.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' no es una fecha con el formato AAAA-MM-DD.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: El almacén 'files' no es un almacén remoto.

$ pasejo org devices --org nope
? 1
Error: Ninguna organización registrada coincide con 'nope'.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: No hay ningún servidor registrado en https://other.example. Ejecute 'pasejo server login https://other.example'.

$ pasejo org devices
? 1
Error: Este dispositivo no ha iniciado sesión en https://pasejo.example. Ejecute 'pasejo server login https://pasejo.example'.

$ pasejo server login https://other.example --emergency
? 1
Error: El inicio de sesión de emergencia necesita un dispositivo ya registrado en https://other.example.

```
