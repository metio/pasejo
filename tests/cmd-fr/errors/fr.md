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
Error: Valeur 'tru' invalide pour l'option 'ignore-missing-identities' : attendu l'un de true, false, 1, 0, yes, no, y, n

$ pasejo store decrypt
? 1
Error: Déchiffrement refusé : cela affichera tous les secrets en clair sur stdout. Passez --yes-i-know pour confirmer.

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Le navigateur 'chrome' nécessite --extension-id car les identifiants d'extension des navigateurs de la famille Chromium sont dérivés de la clé publique de l'extension décompressée.

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' n'est pas un nom d'hôte de messagerie native valide pour firefox : il doit être composé de parties séparées par des points, faites de lettres, chiffres et traits de soulignement, en minuscules pour tout navigateur sauf firefox.

$ pasejo browser install-manifest --browser firefox --extension-id 'evil" & calc' --print --binary /usr/local/bin/pasejo
? 1
Error: --extension-id 'evil" & calc' n'est pas un identifiant d'extension valide : il ne peut contenir que des lettres ASCII, des chiffres, '@', '.', '_', '-' et des accolades.

$ pasejo otp add --type totp --secret JBSWY3DPEHPK3PX0 not-base32
? 1
Error: Le secret du mot de passe à usage unique doit être une chaîne base32 non vide (lettres A-Z et chiffres 2-7)

$ pasejo otp add --url otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PX0 not-base32-url
? 1
Error: L'URL otpauth ne décrit pas un mot de passe à usage unique valide

```
