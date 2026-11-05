# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinataires

recipient-added = Destinataire pour '{ $public_key }' ajouté
recipient-removed = Destinataire pour '{ $public_key }' supprimé
recipient-does-not-exist-ignored = Le destinataire pour '{ $public_key }' n'existe pas dans le coffre – ignoré
recipient-unsupported-key-skipped = Clé { $provider } de type '{ $key_type }' de l'utilisateur '{ $username }' ignorée : age ne peut pas chiffrer pour elle

## Secrets

secret-added = Secret ajouté à '{ $secret_path }'
secret-edited = Secret modifié à '{ $secret_path }'
secret-generated = Secret généré à '{ $secret_path }'
secret-copied = Secret copié de '{ $source_path }' vers '{ $target_path }'
secret-moved = Secret déplacé de '{ $source_path }' vers '{ $target_path }'
secret-removed = Secret supprimé à '{ $secret_path }'
secret-show-as-qrcode = Affichage du secret à '{ $secret_path }' en code QR
secret-show-as-text = Affichage du secret à '{ $secret_path }' en texte
secret-copy-into-clipboard = Secret '{ $secret_path }' copié dans le presse-papiers — sera effacé dans { $duration } (Ctrl-C pour effacer maintenant)

## Mots de passe à usage unique

one-time-password-added = Mot de passe à usage unique ajouté à '{ $password_path }'
one-time-password-copied = Mot de passe à usage unique copié de '{ $source_path }' vers '{ $target_path }'
one-time-password-moved = Mot de passe à usage unique déplacé de '{ $source_path }' vers '{ $target_path }'
one-time-password-removed = Mot de passe à usage unique supprimé à '{ $password_path }'
one-time-password-show = Affichage du mot de passe à usage unique à '{ $password_path }'
one-time-password-copy-into-clipboard = Mot de passe à usage unique '{ $password_path }' copié dans le presse-papiers — sera effacé dans { $duration } (Ctrl-C pour effacer maintenant)

## Identités

identity-added = Identité avec le fichier '{ $identity_file }' ajoutée
identity-removed = Identité avec le fichier '{ $identity_file }' supprimée
no-identities-exist-yet = Aucune identité dans le coffre '{ $store_name }' pour le moment. Veuillez en ajouter une avec 'pasejo identity add ...'.

## Coffres

store-add-success = Coffre '{ $store_name }' ajouté à '{ $store_path }'
store-set-default = Le coffre '{ $store_name }' est maintenant le coffre par défaut
store-remove-success = Coffre '{ $store_name }' supprimé
store-remove-success-with-data = Magasin '{ $store_name }' supprimé et son fichier de données chiffré effacé

## Exécution des hooks

execute-pull-hooks = Exécution des hooks pull pour le coffre '{ $store_name }'
execute-push-hooks = Exécution des hooks push pour le coffre '{ $store_name }'
hook-run-nothing-selected = Rien à exécuter : utilisez --pull et/ou --push pour choisir les hooks à exécuter.
export-plaintext-warning = Avertissement : l'export contient tous les secrets en clair.
export-hotp-omitted = Avertissement : Bitwarden ne peut pas stocker de mots de passe à usage unique HOTP, ceux-ci ne font donc pas partie de l'export : { $names }
export-orphaned-otp-omitted = Avertissement : ces mots de passe à usage unique n'ont pas de secret du même nom et ne font pas partie de l'export : { $names }
export-written-to-file = Export écrit dans '{ $path }'

## Conflits de fusion

merge-conflict-recipient-names = Conflit de fusion pour le destinataire avec la clé publique '{ $public_key }' : les noms '{ $first_name }' et '{ $second_name }' diffèrent
merge-conflict-recipient-removed-and-renamed = Conflit de fusion pour le destinataire avec la clé publique '{ $public_key }' : le destinataire a été supprimé dans une version et renommé en '{ $new_name }' dans l'autre
merge-conflict-values = Conflit de fusion pour { $value_type } à '{ $secret_path }' : les valeurs diffèrent entre les deux versions
merge-conflict-removed-and-modified = Conflit de fusion pour { $value_type } à '{ $secret_path }' : { $value_type } a été supprimé dans une version et modifié dans l'autre

## Sortie de liste (stdout)

list-global-identity = global : { $identity_file }
list-store-identity = coffre : { $identity_file }
list-store-identity-also-global = coffre : { $identity_file } (également enregistrée globalement)
list-global-pull-hook = pull global : { $command }
list-global-push-hook = push global : { $command }
list-store-pull-hook = pull coffre : { $command }
list-store-push-hook = push coffre : { $command }
list-store = { $is_default ->
    [true] { $store_name } : { $store_path } (par défaut)
   *[false] { $store_name } : { $store_path }
  }
password-strength = { $secret_path } : { $score }/100
secret-search-match =
    { $key } :
    { $value }

## Presse-papiers

clipboard-read-for-compare-failed = Échec de la lecture du presse-papiers pour comparaison : { $error }
clipboard-ctrlc-handler-install-failed = Échec de l'installation du gestionnaire Ctrl-C : { $error }. Le presse-papiers ne sera effacé qu'après le délai configuré.
clipboard-clear-failed = Échec de l'effacement du presse-papiers : { $error }
clipboard-manual-clear-required = Le presse-papiers n'a pas pu être effacé automatiquement — veuillez l'effacer manuellement maintenant.
clipboard-notification-dispatch-failed = Échec de l'affichage de la notification d'effacement du presse-papiers : { $error }
clipboard-drop-clear-failed = Échec de l'effacement du presse-papiers pendant le nettoyage : { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] Presse-papiers effacé (annulé)
   *[false] Presse-papiers effacé
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] Presse-papiers inchangé (vous avez copié autre chose) (annulé)
   *[false] Presse-papiers inchangé (vous avez copié autre chose)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] Presse-papiers effacé (contenu non vérifiable) (annulé)
   *[false] Presse-papiers effacé (contenu non vérifiable)
  }
clipboard-notification-failed = { $cancelled ->
    [true] Échec de l'effacement du presse-papiers ! Veuillez l'effacer manuellement. (annulé)
   *[false] Échec de l'effacement du presse-papiers ! Veuillez l'effacer manuellement.
  }

## Invites

prompt-enter-secret = Saisir le secret pour { $secret_path } :
prompt-could-not-read-secret = Impossible de lire le secret pour { $secret_path }
prompt-overwrite-secret = Écraser le secret existant ?
prompt-remove-secret = Supprimer le secret existant ?
prompt-overwrite-one-time-password = Écraser le mot de passe à usage unique existant ?
prompt-remove-one-time-password = Supprimer le mot de passe à usage unique existant ?

## Messages d'erreur

error-cannot-get-user-confirmation = Impossible d'obtenir la confirmation de l'utilisateur
error-no-confirmation-from-non-terminal = Impossible d'obtenir la confirmation depuis une entrée non terminale. Utilisez --force pour ignorer la confirmation.
error-could-not-load-configuration = Impossible de charger la configuration
error-store-does-not-exist = Le coffre nommé '{ $store_name }' n'existe pas dans la configuration
error-invalid-line-number = '{ $input }' n'est pas un numéro de ligne valide
error-line-number-must-not-be-zero = Le numéro de ligne ne doit pas être 0. Utilisez 1 pour la première ligne, -1 pour la dernière
error-invalid-count = '{ $input }' n'est pas un compte valide
error-count-must-not-be-zero = Le compte ne doit pas être 0. Utilisez 1 pour ignorer la première ligne
error-file-does-not-exist = Le fichier '{ $input }' n'existe pas
error-secret-already-exists = Un secret existe déjà à { $secret_path }. Utilisez --force pour l'écraser.
error-secret-already-exists-overwrite-or-inplace = Un secret existe déjà à { $secret_path }. Utilisez --force pour l'écraser entièrement ou --inplace pour modifier sa première ligne sur place.
error-password-length-zero = La longueur du mot de passe ne doit pas être 0
error-password-length-too-short = La longueur du mot de passe est trop courte pour les types de caractères requis
error-password-no-character-kinds = Au moins un type de caractère doit être activé
error-password-generation-failed = Impossible de générer un mot de passe : { $reason }
error-no-secret-found = Aucun secret trouvé à '{ $secret_path }'
error-not-allowed-to-remove-secret = Suppression du secret à { $secret_path } interdite. Utilisez --force pour l'écraser.
error-secret-does-not-exist-for-edit = Le secret à { $secret_path } n'existe pas. Utilisez 'pasejo secret add' pour le créer.
error-one-time-password-already-exists = Un mot de passe à usage unique existe déjà à { $password_path }. Utilisez --force pour l'écraser.
error-not-allowed-to-remove-one-time-password = Suppression du mot de passe à usage unique à { $password_path } interdite. Utilisez --force pour l'écraser.
error-no-one-time-password-found = Aucun mot de passe à usage unique trouvé à '{ $password_path }'
error-invalid-one-time-password-secret = Le secret du mot de passe à usage unique doit être une chaîne base32 non vide (lettres A-Z et chiffres 2-7)
error-invalid-otpauth-url = L'URL otpauth ne décrit pas un mot de passe à usage unique valide
error-hotp-counter-exhausted = Le compteur HOTP a atteint sa valeur maximale et ne peut plus avancer
error-no-store-in-configuration = Aucun coffre trouvé dans la configuration. Exécutez d'abord 'pasejo store add ...' pour en ajouter un
error-unknown-store = Le magasin '{ $store_name }' n'existe pas dans la configuration. Exécutez 'pasejo store list' pour voir les magasins configurés.
error-decrypt-requires-yes-i-know = Déchiffrement refusé : cela affichera tous les secrets en clair sur stdout. Passez --yes-i-know pour confirmer.
error-no-store-or-global = Aucun coffre trouvé dans la configuration et aucune option --global spécifiée. Exécutez d'abord 'pasejo store add ...' pour en ajouter un
error-store-name-already-exists = Le nom du coffre existe déjà. Veuillez utiliser un autre nom.
error-store-path-is-directory = Impossible d'utiliser un répertoire comme chemin de coffre. Veuillez utiliser un chemin de fichier.
error-cannot-create-store-path = Impossible de créer le chemin du coffre. Veuillez vérifier le chemin et réessayer.
error-cannot-get-store-parent = Impossible d'obtenir le répertoire parent du chemin du coffre. Veuillez vérifier le chemin et réessayer.
error-cannot-identify-store = Impossible d'identifier le coffre. Définissez un coffre par défaut, utilisez --store pour spécifier un coffre, ou --global pour définir l'identité globalement.
error-identity-file-already-added = valeur invalide '{ $file }' pour '--file <FILE>' : le fichier a déjà été ajouté comme identité
error-identity-file-not-known = valeur invalide '{ $file }' pour '--file <FILE>' : le fichier ne correspond à aucune identité connue
error-identity-did-you-mean = Vouliez-vous dire '{ $stored }' ? Les chemins d'identité sont comparés à l'identique.
error-no-identity-files-to-decrypt = Aucun fichier d'identité à déchiffrer. Ajoutez au moins une identité pour terminer l'initialisation du coffre.
error-invalid-ssh-public-key-format = Format de clé publique SSH non valide
error-username-empty = Le nom d'utilisateur ne doit pas être vide
error-username-contains-dotdot = Nom d'utilisateur non valide '{ $username }' : ne doit pas contenir '..'
error-username-invalid-character = Nom d'utilisateur non valide '{ $username }' : doit commencer par une lettre ou un chiffre ASCII et ne contenir que des lettres ASCII, chiffres, '-', '_' et '.'
error-no-public-key-found-in-file = Aucune clé publique trouvée dans '{ $filename }'
error-no-public-keys-for-user = L'utilisateur { $provider } '{ $username }' n'a aucune clé SSH publique à ajouter comme destinataire
error-no-public-key-source = Vous devez spécifier au moins une source pour une clé publique
error-no-qrcode-found = Aucun code QR trouvé dans '{ $qrcode }'
error-failed-to-decode-qrcode = Échec du décodage du code QR dans '{ $qrcode }'
error-qr-sandbox-failed = Impossible d'exécuter le décodeur de code QR dans le bac à sable
error-qr-sandbox-child-signal = Le décodeur de code QR a été terminé par le signal { $signal }
error-qr-sandbox-not-enforced = Le bac à sable du décodeur de code QR n'a pas pu être appliqué ; le noyau en cours d'exécution ne prend pas en charge Landlock
error-cannot-determine-store-name = Impossible de déterminer le nom du coffre
error-cannot-parse-hook-command = Impossible d'analyser la commande : { $command }
error-empty-hook-command = Commande de hook vide : { $command }
error-failed-to-run-hook = Échec de l'exécution du hook { $command }
error-hook-failed-no-detail = le hook { $command } a échoué (sortie { $exit })
error-hook-failed-with-detail = le hook { $command } a échoué (sortie { $exit }) : { $detail }
error-cannot-determine-store-parent-path = Impossible de déterminer le parent du chemin du coffre { $path }
error-store-path-not-utf8 = Impossible de substituer %p dans le jeton { $token } : le chemin du coffre { $path } n'est pas UTF-8 valide
error-merge-conflict-recipients = Conflit de fusion détecté dans les destinataires. Veuillez le résoudre manuellement.
error-merge-conflict = Conflit de fusion détecté dans les { $value_type }s. Veuillez le résoudre manuellement.
error-recipient-not-found-in-store = Destinataire introuvable dans le coffre
error-cannot-remove-last-recipient = Impossible de supprimer le dernier destinataire du magasin. Veuillez ajouter un nouveau destinataire avant de supprimer celui-ci.
error-cannot-decrypt-common-ancestor-store = Impossible de déchiffrer le coffre de l'ancêtre commun
error-cannot-decrypt-current-version-store = Impossible de déchiffrer le coffre de la version actuelle
error-cannot-decrypt-other-version-store = Impossible de déchiffrer le coffre de l'autre version
error-cannot-read-file = Impossible de lire le fichier à '{ $path }'
error-downloading-public-key-failed = Échec du téléchargement de la clé publique depuis { $provider }
error-download-user-not-found = { $provider } n'a aucun utilisateur '{ $username }'
error-download-rate-limited = { $provider } a limité la requête ; patientez un instant et réessayez
error-download-server-error = { $provider } a renvoyé une erreur (HTTP { $status })
error-download-timeout = Délai dépassé lors du téléchargement de la clé publique depuis { $provider } ; vérifiez votre connexion ou augmentez key-download-timeout-seconds
error-cannot-determine-parent-directory = Impossible de déterminer le répertoire parent de { $path }
error-failed-to-create-directory = Échec de la création du répertoire { $path }
error-failed-to-write-file = Échec de l'écriture de { $path }
error-failed-to-fsync-file = Échec de fsync sur { $path }
error-failed-to-rename-file = Échec du renommage de { $from } en { $to }
error-failed-to-create-file = Échec de la création de { $path }
error-could-not-resolve-config-path = Impossible de résoudre le chemin absolu de la configuration
error-could-not-determine-config-path = Impossible de déterminer le chemin de configuration
error-could-not-resolve-store-path = Impossible de résoudre le chemin absolu du coffre
error-could-not-create-config-dir = Impossible de créer le répertoire de configuration
error-could-not-migrate-legacy-config = Impossible de migrer le fichier de configuration hérité
error-could-not-move-file = Impossible de déplacer le fichier
error-could-not-copy-file = Impossible de copier le fichier
error-could-not-remove-source-after-copy = Impossible de supprimer le fichier source après la copie
error-could-not-read-configuration = Impossible de lire la configuration
error-could-not-serialize-migrated-config = Impossible de sérialiser la configuration migrée
error-could-not-store-configuration = Impossible d'enregistrer la configuration
error-could-not-load-migrated-config = Impossible de charger la configuration migrée
error-could-not-serialize-configuration = Impossible de sérialiser la configuration
error-config-not-valid-toml = Le fichier de configuration à { $path } n'est pas un TOML valide
error-config-invalid-integer = Valeur '{ $value }' invalide pour l'option '{ $option }' : un entier non négatif est attendu
error-config-invalid-boolean = Valeur '{ $value }' invalide pour l'option '{ $option }' : attendu l'un de true, false, 1, 0, yes, no, y, n
error-could-not-open-store-for-lock = Impossible d'ouvrir le fichier du coffre pour le verrouillage : { $path }
error-could-not-acquire-store-lock = Impossible d'acquérir le verrou sur le fichier du coffre : { $path }
error-store-lock-wait-timed-out = Délai dépassé en attendant le verrou du fichier du coffre : { $path }
error-cannot-encrypt-store = Impossible de chiffrer le coffre
error-failed-to-run-command = Échec de l'exécution de la commande { $binary }
error-command-exited-with = La commande { $binary } s'est terminée avec { $exit }
error-browser-host-requires-pipe = L'hôte du navigateur nécessite une entrée standard redirigée depuis l'extension du navigateur connectée. Exécutez cette commande via le manifeste de messagerie native de votre navigateur, et non directement depuis un terminal.
error-browser-store-file-missing = Le fichier du coffre n'existe pas : { $path }
error-browser-identity-passphrase-protected = L'hôte du navigateur ne peut pas demander la phrase de passe d'une identité chiffrée. Utilisez une identité non chiffrée ou gérée par un plugin pour ce coffre.
error-browser-message-too-large = Le message de { $length } octets dépasse la limite de messagerie native de { $limit } octets
error-browser-response-too-large = La réponse de { $length } octets dépasse la limite de messagerie native de { $limit } octets
error-browser-install-manifest-no-browser = Spécifiez au moins un --browser pour lequel installer le manifeste de messagerie native.
error-browser-install-manifest-needs-extension-id = Le navigateur '{ $browser }' nécessite --extension-id car les identifiants d'extension des navigateurs de la famille Chromium sont dérivés de la clé publique de l'extension décompressée.
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id ne peut pas être combiné avec plusieurs --browser, car chaque navigateur nécessite son propre identifiant d'extension. Exécutez la commande une fois par navigateur.
error-browser-install-manifest-unsupported-os = L'installation des manifestes de messagerie native n'est pas prise en charge sur '{ $os }'.
error-browser-install-manifest-registry-failed = Impossible d'écrire la clé de registre de native-messaging '{ $key }'
error-browser-install-manifest-no-home-dir = Impossible de déterminer le répertoire personnel de l'utilisateur.
error-browser-install-manifest-no-data-dir = Impossible de déterminer le répertoire de données de l'utilisateur.
error-browser-install-manifest-no-current-exe = Impossible de déterminer le chemin du binaire pasejo en cours d'exécution.
browser-install-manifest-written = Manifeste de messagerie native pour '{ $browser }' écrit dans '{ $path }'
browser-install-manifest-print-wrapper-note = Remarque : le manifeste affiché référence le script wrapper situé à '{ $path }', que cette commande n'a pas installé. Utilisez --binary pour référencer un exécutable existant à la place.
error-browser-install-manifest-invalid-env = L'entrée --env '{ $entry }' n'est pas au format KEY=VALUE.
error-browser-install-manifest-invalid-env-name = Le nom --env '{ $name }' n'est pas un nom de variable d'environnement POSIX valide (doit commencer par une lettre ou un trait de soulignement et ne contenir que des lettres, chiffres et traits de soulignement).
error-browser-install-manifest-invalid-env-value-windows = La valeur --env de '{ $name }' ne peut pas être écrite dans le wrapper Windows : elle contient un guillemet double ou un saut de ligne.
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' n'est pas un nom d'hôte de messagerie native valide pour { $browser } : il doit être composé de parties séparées par des points, faites de lettres, chiffres et traits de soulignement, en minuscules pour tout navigateur sauf firefox.
