```
$ pasejo secret add some-secret
? 1
Error: Aucun coffre trouvé dans la configuration. Exécutez d'abord 'pasejo store add ...' pour en ajouter un

$ pasejo store add --path store --name primary
Coffre 'primary' ajouté à '[CWD]/store'

$ pasejo store add --path other --name primary
? 1
Error: Le nom du coffre existe déjà. Veuillez utiliser un autre nom.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': Le coffre nommé 'missing' n'existe pas dans la configuration

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

$ pasejo store decrypt
? 1
Error: Déchiffrement refusé : cela affichera tous les secrets en clair sur stdout. Passez --yes-i-know pour confirmer.

```
