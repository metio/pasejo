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
clipboard-drop-clear-failed = Cancellazione degli appunti durante la pulizia fallita: { $error }
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

## Richieste

prompt-enter-secret = Inserire il segreto per { $secret_path }:
prompt-could-not-read-secret = Impossibile leggere il segreto per { $secret_path }
prompt-overwrite-secret = Sovrascrivere il segreto esistente?
prompt-remove-secret = Rimuovere il segreto esistente?
prompt-overwrite-one-time-password = Sovrascrivere la password monouso esistente?
prompt-remove-one-time-password = Rimuovere la password monouso esistente?

## Messaggi di errore

error-cannot-get-user-confirmation = Impossibile ottenere la conferma dell'utente
error-no-confirmation-from-non-terminal = Impossibile ottenere la conferma da input non terminale. Usare --force per saltare la conferma.
error-could-not-load-configuration = Impossibile caricare la configurazione
error-store-does-not-exist = L'archivio con nome '{ $store_name }' non esiste nella configurazione
error-invalid-line-number = '{ $input }' non è un numero di riga valido
error-line-number-must-not-be-zero = Il numero di riga non deve essere 0. Usare 1 per la prima riga, -1 per l'ultima
error-invalid-count = '{ $input }' non è un conteggio valido
error-count-must-not-be-zero = Il conteggio non deve essere 0. Usare 1 per saltare la prima riga
error-file-does-not-exist = Il file '{ $input }' non esiste
error-secret-already-exists = Il segreto esiste già a { $secret_path }. Usare --force per sovrascriverlo.
error-secret-already-exists-overwrite-or-inplace = Il segreto esiste già a { $secret_path }. Usare --force per sovrascriverlo completamente o --inplace per modificarne solo la prima riga.
error-no-secret-found = Nessun segreto trovato a '{ $secret_path }'
error-not-allowed-to-remove-secret = Rimozione del segreto a { $secret_path } non consentita. Usare --force per sovrascriverlo.
error-secret-does-not-exist-for-edit = Il segreto a { $secret_path } non esiste. Usare 'pasejo secret add' per crearlo.
error-one-time-password-already-exists = La password monouso esiste già a { $password_path }. Usare --force per sovrascriverla.
error-not-allowed-to-remove-one-time-password = Rimozione della password monouso a { $password_path } non consentita. Usare --force per sovrascriverla.
error-no-one-time-password-found = Nessuna password monouso trovata a '{ $password_path }'
error-no-store-in-configuration = Nessun archivio trovato nella configurazione. Eseguire prima 'pasejo store add ...' per aggiungerne uno
error-no-store-or-global = Nessun archivio trovato nella configurazione e nessun flag --global specificato. Eseguire prima 'pasejo store add ...' per aggiungerne uno
error-store-name-already-exists = Il nome dell'archivio esiste già. Usare un nome diverso.
error-store-path-is-directory = Impossibile usare una directory come percorso dell'archivio. Usare un percorso di file.
error-cannot-create-store-path = Impossibile creare il percorso dell'archivio. Verificare il percorso e riprovare.
error-cannot-get-store-parent = Impossibile ottenere la directory padre del percorso dell'archivio. Verificare il percorso e riprovare.
error-cannot-identify-store = Impossibile identificare l'archivio. Impostare un archivio predefinito, usare --store per specificare un archivio o --global per impostare l'identità globalmente.
error-no-identity-files-to-decrypt = Nessun file di identità da decifrare. Aggiungere almeno un'identità per completare l'inizializzazione dell'archivio.
error-invalid-ssh-public-key-format = Formato di chiave pubblica SSH non valido
error-username-empty = Il nome utente non deve essere vuoto
error-username-contains-dotdot = Nome utente non valido '{ $username }': non deve contenere '..'
error-username-invalid-character = Nome utente non valido '{ $username }': deve iniziare con una lettera o cifra ASCII e contenere solo lettere ASCII, cifre, '-', '_' e '.'
error-no-public-key-found-in-file = Nessuna chiave pubblica trovata in '{ $filename }'
error-no-public-key-source = Specificare almeno una fonte per una chiave pubblica
error-no-qrcode-found = Nessun codice QR trovato in '{ $qrcode }'
error-failed-to-decode-qrcode = Decodifica del codice QR in '{ $qrcode }' fallita
error-cannot-determine-store-name = Impossibile determinare il nome dell'archivio
error-cannot-parse-hook-command = Impossibile analizzare il comando: { $command }
error-empty-hook-command = Comando hook vuoto: { $command }
error-failed-to-run-hook = Esecuzione del hook { $command } fallita
error-hook-failed-no-detail = il hook { $command } è fallito (uscita { $exit })
error-hook-failed-with-detail = il hook { $command } è fallito (uscita { $exit }): { $detail }
error-cannot-determine-store-parent-path = Impossibile determinare il padre del percorso dell'archivio { $path }
error-store-path-not-utf8 = Impossibile sostituire %p nel token { $token }: il percorso dell'archivio { $path } non è UTF-8 valido
error-merge-conflict-recipients = Conflitto di unione rilevato nei destinatari. Risolvere manualmente.
error-recipient-not-found-in-store = Destinatario non trovato nell'archivio
error-cannot-decrypt-common-ancestor-store = Impossibile decifrare l'archivio dell'antenato comune
error-cannot-decrypt-current-version-store = Impossibile decifrare l'archivio della versione corrente
error-cannot-decrypt-other-version-store = Impossibile decifrare l'archivio dell'altra versione
error-cannot-read-file = Impossibile leggere il file a '{ $path }'
error-downloading-public-key-failed = Download della chiave pubblica da { $provider } fallito
error-cannot-determine-parent-directory = Impossibile determinare la directory padre di { $path }
error-failed-to-create-directory = Creazione della directory { $path } fallita
error-failed-to-write-file = Scrittura di { $path } fallita
error-failed-to-fsync-file = fsync su { $path } fallito
error-failed-to-rename-file = Rinomina di { $from } in { $to } fallita
error-failed-to-create-file = Creazione di { $path } fallita
error-could-not-resolve-config-path = Impossibile risolvere il percorso assoluto della configurazione
error-could-not-determine-config-path = Impossibile determinare il percorso della configurazione
error-could-not-resolve-store-path = Impossibile risolvere il percorso assoluto dell'archivio
error-could-not-create-config-dir = Impossibile creare la directory di configurazione
error-could-not-migrate-legacy-config = Impossibile migrare il file di configurazione legacy
error-could-not-move-file = Impossibile spostare il file
error-could-not-copy-file = Impossibile copiare il file
error-could-not-remove-source-after-copy = Impossibile rimuovere il file sorgente dopo la copia
error-could-not-read-configuration = Impossibile leggere la configurazione
error-could-not-serialize-migrated-config = Impossibile serializzare la configurazione migrata
error-could-not-store-configuration = Impossibile salvare la configurazione
error-could-not-load-migrated-config = Impossibile caricare la configurazione migrata
error-could-not-serialize-configuration = Impossibile serializzare la configurazione
error-config-not-valid-toml = Il file di configurazione a { $path } non è TOML valido
error-could-not-open-store-for-lock = Impossibile aprire il file dell'archivio per il blocco: { $path }
error-could-not-acquire-store-lock = Impossibile acquisire il blocco sul file dell'archivio: { $path }
error-cannot-encrypt-store = Impossibile cifrare l'archivio
error-failed-to-run-command = Esecuzione del comando { $binary } fallita
error-command-exited-with = Il comando { $binary } è terminato con { $exit }
