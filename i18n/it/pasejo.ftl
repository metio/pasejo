# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinatari

recipient-added = Destinatario per '{ $public_key }' aggiunto
recipient-removed = Destinatario per '{ $public_key }' rimosso
recipient-does-not-exist-ignored = Il destinatario per '{ $public_key }' non esiste nell'archivio – verrà ignorato

## Segreti

secret-added = Segreto aggiunto in '{ $secret_path }'
secret-edited = Segreto modificato in '{ $secret_path }'
secret-generated = Segreto generato in '{ $secret_path }'
secret-copied = Segreto copiato da '{ $source_path }' a '{ $target_path }'
secret-moved = Segreto spostato da '{ $source_path }' a '{ $target_path }'
secret-removed = Segreto rimosso in '{ $secret_path }'
secret-show-as-qrcode = Visualizzazione del segreto in '{ $secret_path }' come codice QR
secret-show-as-text = Visualizzazione del segreto in '{ $secret_path }' come testo
secret-copy-into-clipboard = Segreto '{ $secret_path }' copiato negli appunti — verrà cancellato tra { $duration } (Ctrl-C per cancellare ora)

## Password monouso

one-time-password-added = Password monouso aggiunta in '{ $password_path }'
one-time-password-copied = Password monouso copiata da '{ $source_path }' a '{ $target_path }'
one-time-password-moved = Password monouso spostata da '{ $source_path }' a '{ $target_path }'
one-time-password-removed = Password monouso rimossa in '{ $password_path }'
one-time-password-show = Visualizzazione della password monouso in '{ $password_path }'
one-time-password-copy-into-clipboard = Password monouso '{ $password_path }' copiata negli appunti — verrà cancellata tra { $duration } (Ctrl-C per cancellare ora)

## Identità

identity-added = Identità con file '{ $identity_file }' aggiunta
identity-removed = Identità con file '{ $identity_file }' rimossa
no-identities-exist-yet = Nessuna identità nell'archivio '{ $store_name }'. Aggiungerne una con 'pasejo identity add ...'.

## Archivi

store-add-success = Archivio '{ $store_name }' aggiunto in '{ $store_path }'
store-set-default = L'archivio '{ $store_name }' è ora quello predefinito
store-remove-success = Archivio '{ $store_name }' rimosso

## Esecuzione hook

execute-pull-hooks = Esecuzione degli hook di pull per l'archivio '{ $store_name }'
execute-push-hooks = Esecuzione degli hook di push per l'archivio '{ $store_name }'

## Conflitti di unione

merge-conflict-recipient-names = Conflitto di unione per destinatario con chiave pubblica '{ $public_key }': i nomi '{ $first_name }' e '{ $second_name }' differiscono
merge-conflict-recipient-removed-and-renamed = Conflitto di unione per destinatario con chiave pubblica '{ $public_key }': il destinatario è stato rimosso in una versione e rinominato in '{ $new_name }' nell'altra
merge-conflict-values = Conflitto di unione per { $value_type } in '{ $secret_path }': i valori differiscono tra le due versioni
merge-conflict-removed-and-modified = Conflitto di unione per { $value_type } in '{ $secret_path }': { $value_type } è stato rimosso in una versione e modificato nell'altra

## Output di elenco (stdout)

list-global-identity = globale: { $identity_file }
list-store-identity = archivio: { $identity_file }
list-global-pull-hook = pull globale: { $command }
list-global-push-hook = push globale: { $command }
list-store-pull-hook = pull archivio: { $command }
list-store-push-hook = push archivio: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (predefinito)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## Appunti

clipboard-read-for-compare-failed = Lettura degli appunti per confronto fallita: { $error }
clipboard-ctrlc-handler-install-failed = Installazione del gestore Ctrl-C fallita: { $error }. Gli appunti verranno cancellati solo dopo il timeout configurato.
clipboard-clear-failed = Cancellazione degli appunti fallita: { $error }
clipboard-manual-clear-required = Gli appunti non sono stati cancellati automaticamente — cancellarli manualmente ora.
clipboard-notification-dispatch-failed = Visualizzazione della notifica di cancellazione degli appunti fallita: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] Appunti cancellati (annullato)
   *[false] Appunti cancellati
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] Appunti invariati (hai copiato qualcos'altro) (annullato)
   *[false] Appunti invariati (hai copiato qualcos'altro)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] Appunti cancellati (contenuto non verificabile) (annullato)
   *[false] Appunti cancellati (contenuto non verificabile)
  }
clipboard-notification-failed = { $cancelled ->
    [true] Cancellazione degli appunti fallita! Cancellarli manualmente. (annullato)
   *[false] Cancellazione degli appunti fallita! Cancellarli manualmente.
  }
