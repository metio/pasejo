# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinataires

recipient-added = Destinataire pour '{ $public_key }' ajouté
recipient-removed = Destinataire pour '{ $public_key }' supprimé
recipient-does-not-exist-ignored = Le destinataire pour '{ $public_key }' n'existe pas dans le coffre – ignoré

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

## Exécution des hooks

execute-pull-hooks = Exécution des hooks pull pour le coffre '{ $store_name }'
execute-push-hooks = Exécution des hooks push pour le coffre '{ $store_name }'

## Conflits de fusion

merge-conflict-recipient-names = Conflit de fusion pour le destinataire avec la clé publique '{ $public_key }' : les noms '{ $first_name }' et '{ $second_name }' diffèrent
merge-conflict-recipient-removed-and-renamed = Conflit de fusion pour le destinataire avec la clé publique '{ $public_key }' : le destinataire a été supprimé dans une version et renommé en '{ $new_name }' dans l'autre
merge-conflict-values = Conflit de fusion pour { $value_type } à '{ $secret_path }' : les valeurs diffèrent entre les deux versions
merge-conflict-removed-and-modified = Conflit de fusion pour { $value_type } à '{ $secret_path }' : { $value_type } a été supprimé dans une version et modifié dans l'autre

## Sortie de liste (stdout)

list-global-identity = global : { $identity_file }
list-store-identity = coffre : { $identity_file }
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
