```
$ pasejo recipient list
? 1
Error: Le coffre 'team' est distant ; les destinataires, hooks et fusions ne s'y appliquent pas.

$ pasejo hook get
? 1
Error: Le coffre 'team' est distant ; les destinataires, hooks et fusions ne s'y appliquent pas.

$ pasejo store grant bob@example.com reader --expires 2026-02-30
? 2
error: invalid value '2026-02-30' for '--expires <EXPIRES>': '2026-02-30' n'est pas une date au format AAAA-MM-JJ.

For more information, try '--help'.

$ pasejo store rekey --store files
? 1
Error: Le coffre 'files' n'est pas un coffre distant.

$ pasejo org devices --org nope
? 1
Error: Aucune organisation enregistrée ne correspond à 'nope'.

$ pasejo org join https://other.example fedcba9876543210fedcba9876543210
? 1
Error: Aucun serveur n'est enregistré à https://other.example. Lancez 'pasejo server login https://other.example'.

$ pasejo org devices
? 1
Error: Cet appareil n'est pas connecté à https://pasejo.example. Lancez 'pasejo server login https://pasejo.example'.

$ pasejo server login https://other.example --emergency
? 1
Error: La connexion d'urgence nécessite un appareil déjà enregistré auprès de https://other.example.

```
