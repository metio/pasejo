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
error-editor-launch-failed = Impossible de démarrer l'éditeur '{ $editor }'
error-editor-exited-unsuccessfully = L'éditeur '{ $editor }' ne s'est pas terminé correctement, le secret n'a donc pas été modifié
error-editor-file-not-utf8 = Le secret modifié n'est pas en UTF-8 valide
error-editor-file-create-failed = Impossible de créer un fichier temporaire pour l'éditeur dans '{ $path }'
editor-file-in-temp-dir = Ni $XDG_RUNTIME_DIR ni /dev/shm n'est disponible, le secret est donc écrit dans un fichier dans '{ $path }' pendant que l'éditeur est ouvert. Ce répertoire peut être stocké sur disque.
editor-file-cleanup-failed = Impossible d'écraser et de supprimer le fichier temporaire de l'éditeur '{ $path }' : { $error }
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
error-no-field-found = Le secret '{ $secret_path }' n'a pas de champ '{ $field }'
error-no-line-found = Le secret '{ $secret_path }' n'a pas de ligne { $line }
error-value-unresolved = Impossible de résoudre la valeur de '{ $name }'
error-env-assignment-invalid = '{ $input }' n'est pas de la forme VARIABLE=SECRET
error-env-field-assignment-invalid = '{ $input }' n'est pas de la forme VARIABLE=SECRET=CHAMP
error-env-variable-name-invalid = '{ $variable }' n'est pas un nom de variable d'environnement valide. Utilisez des lettres, des chiffres et des tirets bas, sans commencer par un chiffre
error-env-variable-duplicate = La variable d'environnement { $variable } est affectée plus d'une fois
error-env-reference-line-and-field = La référence pour { $variable } définit à la fois 'line' et 'field'
error-resolve-input-invalid = Impossible de lire les références aux secrets depuis l'entrée standard
exec-value-too-short-to-redact = La valeur de { $variable } fait moins de { $length } caractères et n'est pas masquée
error-browser-host-requires-pipe = L'hôte du navigateur nécessite une entrée standard redirigée depuis l'extension du navigateur connectée. Exécutez cette commande via le manifeste de messagerie native de votre navigateur, et non directement depuis un terminal.
error-browser-store-file-missing = Le fichier du coffre n'existe pas : { $path }
error-browser-identity-passphrase-protected = L'hôte du navigateur ne peut pas demander la phrase de passe d'une identité chiffrée. Utilisez une identité non chiffrée ou gérée par un plugin pour ce coffre.
prompt-browser-host-pinentry = PIN ou phrase de passe
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
error-browser-install-manifest-invalid-extension-id = --extension-id '{ $id }' n'est pas un identifiant d'extension valide : il ne peut contenir que des lettres ASCII, des chiffres, '@', '.', '_', '-' et des accolades.
error-browser-release-log-lock = Impossible de verrouiller le journal des transmissions au navigateur : { $path }
error-browser-release-log-no-data-dir = Impossible de déterminer le répertoire de données du journal des transmissions au navigateur.
error-browser-release-log-failed = La transmission n'a pas pu être consignée dans le journal des transmissions au navigateur ; les identifiants n'ont donc pas été envoyés.
error-browser-caller-not-allowed = L'extension de navigateur '{ $caller }' n'est pas autorisée à utiliser cet hôte pasejo. Exécutez `pasejo browser install-manifest` pour cette extension ou ajoutez-la avec --allowed-caller.
error-browser-caller-unnamed = Le navigateur n'a pas indiqué quelle extension a démarré cet hôte pasejo ; elle ne peut donc pas être vérifiée avec --allowed-caller.
browser-log-empty = Aucun identifiant n'a encore été transmis à un navigateur.
browser-log-unreadable-lines = Lignes illisibles ignorées dans le journal des transmissions au navigateur : { $count }
browser-log-header-time = Heure
browser-log-header-domain = Domaine
browser-log-header-store = Coffre
browser-log-header-secret = Secret
browser-log-header-caller = Appelant

## Remote stores
error-invalid-date = '{ $value }' n'est pas une date au format AAAA-MM-JJ.
error-remote-actions-failed = Certaines actions en attente ont échoué.
error-remote-bad-request = Le serveur n'a pas accepté la requête.
error-remote-checkpoint-key-changed = La clé de checkpoint de { $url } diffère de celle enregistrée lors de la première connexion. Le serveur a peut-être été remplacé ; consultez ses opérateurs avant de vous reconnecter.
error-remote-concurrent-change = Quelqu'un d'autre a modifié '{ $name }' au même moment. Relancez la commande pour appliquer votre modification à la nouvelle version.
error-remote-conflict = Le serveur détient une version plus récente. Relancez la commande.
error-remote-device-user-not-member = { $email } n'est pas membre de l'organisation. Ajoutez cette personne d'abord avec 'pasejo org role { $email } member'.
error-remote-fingerprint-mismatch = L'empreinte ne correspond pas. Le serveur indique { $expected }.
error-remote-fingerprint-not-confirmed = L'empreinte n'a pas été confirmée.
error-remote-fingerprint-required = Comparez l'empreinte { $expected } par un autre canal, puis passez-la avec --fingerprint.
error-remote-forbidden = Le serveur a refusé : vous n'avez pas l'autorisation nécessaire.
error-remote-invalid-id = '{ $value }' n'est pas un identifiant valide.
error-remote-invalid-recipient = '{ $recipient }' n'est pas un destinataire age vers lequel le serveur peut chiffrer.
error-remote-login-denied = La connexion a été refusée dans le navigateur.
error-remote-login-expired = La connexion n'a pas été approuvée à temps. Relancez 'pasejo server login'.
error-remote-needs-second-admin = Cette modification nécessite la signature d'un second administrateur.
error-remote-no-data-dir = Aucun répertoire de données local n'existe pour conserver les clés de cet appareil. Définissez PASEJO_DATA_DIR.
error-remote-no-org = Aucune organisation n'est enregistrée. Lancez 'pasejo org create' ou 'pasejo org join'.
error-remote-no-recovery-recipient = L'organisation { $org } n'a pas de destinataire de récupération.
error-remote-no-server = Aucun serveur n'est enregistré. Lancez 'pasejo server login URL'.
error-remote-no-session = Cet appareil n'est pas connecté.
error-remote-not-found = Le serveur ne connaît pas ce qui a été demandé.
error-remote-not-logged-in = Cet appareil n'est pas connecté à { $url }. Lancez 'pasejo server login { $url }'.
error-remote-org-ambiguous = Plusieurs organisations correspondent à '{ $org }'. Passez l'identifiant de l'organisation.
error-remote-org-required = Plusieurs organisations sont enregistrées. Choisissez-en une avec --org.
error-remote-pending-device = Cet appareil attend l'approbation d'un administrateur ou d'un approbateur de l'organisation.
error-remote-rate-limited = Le serveur demande d'attendre avant de réessayer.
error-remote-recipient-required = Aucune identité globale ne fournit de destinataire age pour cet appareil. Passez-en un avec --recipient.
error-remote-rejected = Le journal des adhésions de l'organisation a refusé la modification.
error-remote-server-ambiguous = Plusieurs serveurs sont enregistrés. Indiquez l'URL du serveur.
error-remote-server-failed = Le serveur n'a pas pu répondre à la requête.
error-remote-session-rejected = Le serveur n'accepte plus la session de cet appareil. Relancez 'pasejo server login'.
error-remote-signing-key-unreadable = La clé de signature de l'appareil à { $path } ne peut pas être lue.
error-remote-store-command-not-applicable = Le coffre '{ $store_name }' est distant ; les destinataires, hooks et fusions ne s'y appliquent pas.
error-remote-store-name-taken = Un coffre nommé '{ $name }' existe déjà.
error-remote-store-required = Le coffre '{ $store_name }' n'est pas un coffre distant.
error-remote-team-exists = Une équipe nommée '{ $name }' existe déjà.
error-remote-too-large = La requête est plus volumineuse que ce que le serveur accepte.
error-remote-unavailable = Il manque à cet appareil ce dont la requête a besoin.
error-remote-unknown-device = Aucun appareil de ce type : { $device }.
error-remote-unknown-org = Aucune organisation enregistrée ne correspond à '{ $org }'.
error-remote-unknown-proposal = Aucune proposition ouverte n'a l'identifiant { $proposal }.
error-remote-unknown-server = Aucun serveur n'est enregistré à { $url }. Lancez 'pasejo server login { $url }'.
error-remote-unknown-store = L'organisation n'a pas de coffre { $store }.
error-remote-unknown-team = L'organisation n'a pas d'équipe nommée '{ $name }'.
error-remote-unknown-user = Personne dans l'organisation n'utilise l'adresse e-mail { $email }.
error-remote-unreachable = Le serveur est injoignable.
error-remote-verification-failed = Ce que le serveur a renvoyé ne se vérifie pas. Ne faites pas confiance à ce serveur tant que ses opérateurs n'en ont pas expliqué la raison.
error-remote-workload-token-unreadable = Le jeton de charge de travail à { $path } ne peut pas être lu.
error-remote-wrong-object-kind = L'entrée '{ $name }' contient des données d'un type inattendu.
prompt-remote-confirm-fingerprint = L'empreinte { $fingerprint } correspond-elle à celle affichée de l'autre côté ?
remote-action-approve-device = approuver l'appareil '{ $label }' de { $email } (empreinte { $fingerprint })
remote-action-cosign-checkpoint = cosigner le checkpoint d'audit { $seq }
remote-action-join-member = ajouter { $email } comme membre
remote-action-remove-member = retirer { $user } de l'organisation
remote-action-revoke-grant = révoquer l'accès de { $principal } au coffre { $store }
remote-action-rotate-epoch = renouveler les clés du coffre { $store }
remote-action-sync-team = mettre à jour les membres de l'équipe { $team }
remote-action-wrap-keys = donner à { $count } appareils les clés du coffre { $store }
remote-device-state-active = actif
remote-device-state-pending = en attente
remote-action-done = fait : { $action }
remote-action-skipped = ignoré : { $action } ({ $reason })
remote-action-failed = échec : { $action } : { $error }
remote-action-pending = en attente : { $action }
remote-audit-line = { $seq }  { $time }  { $actor }  { $action }  { $outcome }  { $store }  { $detail }
remote-device-line = { $id }  { $label }  { $email }  { $fingerprint }  { $state }
remote-proposal-line = { $proposal }  { $kind }  { $author }
remote-store-line = { $store }  { $label }  { $role }  { $local }
remote-rotation-item = { $name }
remote-status-server = { $url } : appareil { $device } ({ $label }), empreinte { $fingerprint }, { $session ->
    [yes] connecté
   *[other] non connecté
}
remote-status-org = - organisation { $name } ({ $org }), empreinte { $fingerprint }
remote-audit-verified = { $events } événements d'audit et { $checkpoints } checkpoints vérifiés.
remote-change-proposed = La modification { $kind } nécessite un second administrateur. La proposition { $proposal } attend 'pasejo org cosign'.
remote-change-recorded = { $kind } inscrit dans le journal des adhésions.
remote-device-active = Cet appareil est actif dans { $org }.
remote-device-pending = Cet appareil attend une approbation dans { $org }. Donnez à un administrateur ou approbateur son identifiant { $device } et son empreinte { $fingerprint }.
remote-device-fingerprint = L'empreinte de cet appareil est { $fingerprint }.
remote-logged-in = Connecté à { $url } en tant que { $email }.
remote-logged-out = Déconnecté de { $url }.
remote-login-visit = Ouvrez { $url } dans un navigateur et confirmez le code { $code }.
remote-no-actions = Aucune action en attente.
remote-no-proposals = Aucune proposition ouverte.
remote-no-servers = Aucun serveur n'est enregistré.
remote-no-stores = L'organisation n'a aucun coffre visible pour cet appareil.
remote-org-created = Organisation { $name } créée avec l'identifiant { $org } et l'empreinte { $fingerprint }.
remote-rotation-access-continues = { $user } a toujours accès au coffre.
remote-rotation-nothing = { $user } n'a rien récupéré qui doive encore être renouvelé.
remote-settings-unchanged = Les paramètres ont déjà ces valeurs.
remote-automatic-approval-warning = Avec l'approbation automatique, `pasejo org actions --run` ajoute chaque appareil qu'un membre de l'organisation inscrit, sans que personne ne compare son empreinte. Un serveur ou un fournisseur d'identité compromis peut alors inscrire son propre appareil et recevoir les clés des magasins. Vérifiez régulièrement `pasejo org devices`.
remote-device-approved-automatically = L'appareil '{ $label }' a été approuvé sans que personne ne compare son empreinte, car l'organisation approuve les appareils automatiquement.
remote-store-added = Coffre distant { $label } enregistré sous '{ $name }'.
remote-store-created = Coffre distant '{ $name }' créé avec l'identifiant { $store }.
remote-store-migrated = { $count } entrées de '{ $name }' copiées sur le serveur. Le coffre désigne désormais la copie distante ; le fichier { $path } est inchangé.
remote-store-recovered = { $count } entrées de { $label } récupérées dans { $path }.
remote-store-rekeyed = { $count } entrées réécrites avec les clés actuelles.

## Remote administration
error-remote-emergency-unregistered = La connexion d'urgence nécessite un appareil déjà enregistré auprès de { $url }.
error-remote-unknown-review = Aucune revue d'accès n'a l'identifiant { $review }.
error-remote-not-under-review = { $principal } n'a aucune autorisation dans la revue { $review }.
remote-emergency-logged-in = Connecté à { $url } en tant que { $email } via la fenêtre d'urgence, jusqu'à { $expires }.
remote-scim-token-created = Jeton SCIM { $id } créé. Il est affiché ci-dessous une seule fois et ne pourra plus être montré.
remote-scim-token-line = { $id }  { $created }
remote-no-scim-tokens = Aucun jeton SCIM valide.
remote-scim-token-revoked = Jeton SCIM { $id } révoqué.
remote-review-opened = Revue d'accès { $review } ouverte pour le coffre { $store }, échéance le { $due }.
remote-review-line = { $review }  { $store }  { $due }  { $state }
remote-review-item = - { $principal }  { $role }  { $decision }
remote-review-state-open = ouverte
remote-review-state-complete = terminée
remote-review-decision-keep = conserver
remote-review-decision-revoke = révoquer
remote-review-decision-pending = en attente
remote-no-reviews = Aucune revue d'accès.
remote-review-decided = « { $decision } » enregistré pour { $principal } dans la revue { $review }.
remote-review-revokes-pending = Les décisions de révocation prennent effet lorsqu'un administrateur de l'organisation ou du coffre exécute 'pasejo org actions --run'.
