# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinatari

recipient-added = Destinatario per '{ $public_key }' aggiunto
recipient-removed = Destinatario per '{ $public_key }' rimosso
recipient-does-not-exist-ignored = Il destinatario per '{ $public_key }' non esiste nell'archivio – verrà ignorato
recipient-unsupported-key-skipped = Chiave { $provider } di tipo '{ $key_type }' dell'utente '{ $username }' ignorata: age non può cifrare per essa

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
store-remove-success-with-data = Archivio '{ $store_name }' rimosso e il suo file di dati cifrato eliminato

## Esecuzione hook

execute-pull-hooks = Esecuzione degli hook di pull per l'archivio '{ $store_name }'
execute-push-hooks = Esecuzione degli hook di push per l'archivio '{ $store_name }'
hook-run-nothing-selected = Niente da eseguire: usare --pull e/o --push per scegliere quali hook eseguire.
export-plaintext-warning = Attenzione: l'esportazione contiene ogni segreto in chiaro.
export-hotp-omitted = Attenzione: Bitwarden non può memorizzare password monouso HOTP, quindi queste non fanno parte dell'esportazione: { $names }
export-orphaned-otp-omitted = Attenzione: queste password monouso non hanno un segreto con lo stesso nome e non fanno parte dell'esportazione: { $names }
export-written-to-file = Esportazione scritta in '{ $path }'

## Conflitti di unione

merge-conflict-recipient-names = Conflitto di unione per destinatario con chiave pubblica '{ $public_key }': i nomi '{ $first_name }' e '{ $second_name }' differiscono
merge-conflict-recipient-removed-and-renamed = Conflitto di unione per destinatario con chiave pubblica '{ $public_key }': il destinatario è stato rimosso in una versione e rinominato in '{ $new_name }' nell'altra
merge-conflict-values = Conflitto di unione per { $value_type } in '{ $secret_path }': i valori differiscono tra le due versioni
merge-conflict-removed-and-modified = Conflitto di unione per { $value_type } in '{ $secret_path }': { $value_type } è stato rimosso in una versione e modificato nell'altra

## Output di elenco (stdout)

list-global-identity = globale: { $identity_file }
list-store-identity = archivio: { $identity_file }
list-store-identity-also-global = archivio: { $identity_file } (registrata anche globalmente)
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
error-editor-launch-failed = Impossibile avviare l'editor '{ $editor }'
error-editor-exited-unsuccessfully = L'editor '{ $editor }' non è terminato correttamente, quindi il segreto non è stato modificato
error-editor-file-not-utf8 = Il segreto modificato non è UTF-8 valido
error-editor-file-create-failed = Impossibile creare un file temporaneo per l'editor in '{ $path }'
editor-file-in-temp-dir = Né $XDG_RUNTIME_DIR né /dev/shm sono disponibili, quindi il segreto viene scritto in un file in '{ $path }' mentre l'editor è aperto. Quella directory potrebbe trovarsi su disco.
editor-file-cleanup-failed = Impossibile sovrascrivere e rimuovere il file temporaneo dell'editor '{ $path }': { $error }
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
error-password-length-zero = La lunghezza della password non può essere 0
error-password-length-too-short = La lunghezza della password è troppo corta per i tipi di carattere richiesti
error-password-no-character-kinds = Deve essere abilitato almeno un tipo di carattere
error-password-generation-failed = Impossibile generare una password: { $reason }
error-no-secret-found = Nessun segreto trovato a '{ $secret_path }'
error-not-allowed-to-remove-secret = Rimozione del segreto a { $secret_path } non consentita. Usare --force per sovrascriverlo.
error-secret-does-not-exist-for-edit = Il segreto a { $secret_path } non esiste. Usare 'pasejo secret add' per crearlo.
error-one-time-password-already-exists = La password monouso esiste già a { $password_path }. Usare --force per sovrascriverla.
error-not-allowed-to-remove-one-time-password = Rimozione della password monouso a { $password_path } non consentita. Usare --force per sovrascriverla.
error-no-one-time-password-found = Nessuna password monouso trovata a '{ $password_path }'
error-invalid-one-time-password-secret = Il segreto della password monouso deve essere una stringa base32 non vuota (lettere A-Z e cifre 2-7)
error-invalid-otpauth-url = L'URL otpauth non descrive una password monouso valida
error-hotp-counter-exhausted = Il contatore HOTP ha raggiunto il valore massimo e non può avanzare
error-no-store-in-configuration = Nessun archivio trovato nella configurazione. Eseguire prima 'pasejo store add ...' per aggiungerne uno
error-unknown-store = L'archivio '{ $store_name }' non esiste nella configurazione. Eseguire 'pasejo store list' per vedere gli archivi configurati.
error-decrypt-requires-yes-i-know = Decifratura rifiutata: stamperà ogni segreto in chiaro su stdout. Passa --yes-i-know per confermare.
error-no-store-or-global = Nessun archivio trovato nella configurazione e nessun flag --global specificato. Eseguire prima 'pasejo store add ...' per aggiungerne uno
error-store-name-already-exists = Il nome dell'archivio esiste già. Usare un nome diverso.
error-store-path-is-directory = Impossibile usare una directory come percorso dell'archivio. Usare un percorso di file.
error-cannot-create-store-path = Impossibile creare il percorso dell'archivio. Verificare il percorso e riprovare.
error-cannot-get-store-parent = Impossibile ottenere la directory padre del percorso dell'archivio. Verificare il percorso e riprovare.
error-cannot-identify-store = Impossibile identificare l'archivio. Impostare un archivio predefinito, usare --store per specificare un archivio o --global per impostare l'identità globalmente.
error-identity-file-already-added = valore non valido '{ $file }' per '--file <FILE>': il file è già stato aggiunto come identità
error-identity-file-not-known = valore non valido '{ $file }' per '--file <FILE>': il file non corrisponde a nessuna identità nota
error-identity-did-you-mean = Intendevi '{ $stored }'? I percorsi delle identità vengono confrontati in modo esatto.
error-no-identity-files-to-decrypt = Nessun file di identità da decifrare. Aggiungere almeno un'identità per completare l'inizializzazione dell'archivio.
error-invalid-ssh-public-key-format = Formato di chiave pubblica SSH non valido
error-username-empty = Il nome utente non deve essere vuoto
error-username-contains-dotdot = Nome utente non valido '{ $username }': non deve contenere '..'
error-username-invalid-character = Nome utente non valido '{ $username }': deve iniziare con una lettera o cifra ASCII e contenere solo lettere ASCII, cifre, '-', '_' e '.'
error-no-public-key-found-in-file = Nessuna chiave pubblica trovata in '{ $filename }'
error-no-public-keys-for-user = L'utente { $provider } '{ $username }' non ha chiavi SSH pubbliche da aggiungere come destinatari
error-no-public-key-source = Specificare almeno una fonte per una chiave pubblica
error-no-qrcode-found = Nessun codice QR trovato in '{ $qrcode }'
error-failed-to-decode-qrcode = Decodifica del codice QR in '{ $qrcode }' fallita
error-qr-sandbox-failed = Impossibile eseguire il decoder di codici QR nella sandbox
error-qr-sandbox-child-signal = Il decoder di codici QR è stato terminato dal segnale { $signal }
error-qr-sandbox-not-enforced = Impossibile applicare la sandbox del decoder di codici QR; il kernel in esecuzione non supporta Landlock
error-cannot-determine-store-name = Impossibile determinare il nome dell'archivio
error-cannot-parse-hook-command = Impossibile analizzare il comando: { $command }
error-empty-hook-command = Comando hook vuoto: { $command }
error-failed-to-run-hook = Esecuzione del hook { $command } fallita
error-hook-failed-no-detail = il hook { $command } è fallito (uscita { $exit })
error-hook-failed-with-detail = il hook { $command } è fallito (uscita { $exit }): { $detail }
error-cannot-determine-store-parent-path = Impossibile determinare il padre del percorso dell'archivio { $path }
error-store-path-not-utf8 = Impossibile sostituire %p nel token { $token }: il percorso dell'archivio { $path } non è UTF-8 valido
error-merge-conflict-recipients = Conflitto di unione rilevato nei destinatari. Risolvere manualmente.
error-merge-conflict = Conflitto di unione rilevato in { $value_type }s. Risolvere manualmente.
error-recipient-not-found-in-store = Destinatario non trovato nell'archivio
error-cannot-remove-last-recipient = Impossibile rimuovere l'ultimo destinatario dall'archivio. Aggiungere un nuovo destinatario prima di rimuovere questo.
error-cannot-decrypt-common-ancestor-store = Impossibile decifrare l'archivio dell'antenato comune
error-cannot-decrypt-current-version-store = Impossibile decifrare l'archivio della versione corrente
error-cannot-decrypt-other-version-store = Impossibile decifrare l'archivio dell'altra versione
error-cannot-read-file = Impossibile leggere il file a '{ $path }'
error-downloading-public-key-failed = Download della chiave pubblica da { $provider } fallito
error-download-user-not-found = { $provider } non ha nessun utente '{ $username }'
error-download-rate-limited = { $provider } ha limitato la richiesta; attendi un momento e riprova
error-download-server-error = { $provider } ha restituito un errore (HTTP { $status })
error-download-timeout = Timeout durante il download della chiave pubblica da { $provider }; controlla la connessione o aumenta key-download-timeout-seconds
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
error-config-invalid-integer = Valore '{ $value }' non valido per l'opzione '{ $option }': atteso un intero non negativo
error-config-invalid-boolean = Valore '{ $value }' non valido per l'opzione '{ $option }': atteso uno tra true, false, 1, 0, yes, no, y, n
error-could-not-open-store-for-lock = Impossibile aprire il file dell'archivio per il blocco: { $path }
error-could-not-acquire-store-lock = Impossibile acquisire il blocco sul file dell'archivio: { $path }
error-store-lock-wait-timed-out = Tempo scaduto in attesa del blocco sul file dell'archivio: { $path }
error-cannot-encrypt-store = Impossibile cifrare l'archivio
error-failed-to-run-command = Esecuzione del comando { $binary } fallita
error-command-exited-with = Il comando { $binary } è terminato con { $exit }
error-browser-host-requires-pipe = L'host del browser richiede uno stdin reindirizzato dall'estensione del browser connessa. Eseguire questo comando tramite il manifest di messaggistica nativa del browser, non direttamente da un terminale.
error-browser-store-file-missing = Il file dell'archivio non esiste: { $path }
error-browser-identity-passphrase-protected = L'host del browser non può chiedere la passphrase di un'identità cifrata. Usa un'identità non cifrata o basata su un plugin per questo archivio.
error-browser-message-too-large = Il messaggio di { $length } byte supera il limite di messaggistica nativa di { $limit } byte
error-browser-response-too-large = La risposta di { $length } byte supera il limite di messaggistica nativa di { $limit } byte
error-browser-install-manifest-no-browser = Specificare almeno un --browser per cui installare il manifest di messaggistica nativa.
error-browser-install-manifest-needs-extension-id = Il browser '{ $browser }' richiede --extension-id perché gli ID delle estensioni dei browser basati su Chromium sono derivati dalla chiave pubblica dell'estensione decompressa.
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id non può essere combinato con più di un --browser, perché ogni browser richiede il proprio id estensione. Eseguire il comando una volta per browser.
error-browser-install-manifest-unsupported-os = L'installazione dei manifest di messaggistica nativa non è supportata su '{ $os }'.
error-browser-install-manifest-registry-failed = Impossibile scrivere la chiave di registro di native-messaging '{ $key }'
error-browser-install-manifest-no-home-dir = Impossibile determinare la directory home dell'utente.
error-browser-install-manifest-no-data-dir = Impossibile determinare la directory dati dell'utente.
error-browser-install-manifest-no-current-exe = Impossibile determinare il percorso del binario pasejo in esecuzione.
browser-install-manifest-written = Manifest di messaggistica nativa per '{ $browser }' scritto in '{ $path }'
browser-install-manifest-print-wrapper-note = Nota: il manifest stampato fa riferimento allo script wrapper in '{ $path }', che questo comando non ha installato. Usare --binary per fare riferimento a un eseguibile esistente.
error-browser-install-manifest-invalid-env = La voce --env '{ $entry }' non è nella forma KEY=VALUE.
error-browser-install-manifest-invalid-env-name = Il nome --env '{ $name }' non è un nome valido per una variabile d'ambiente POSIX (deve iniziare con una lettera o un trattino basso e contenere solo lettere, cifre e trattini bassi).
error-browser-install-manifest-invalid-env-value-windows = Il valore --env di '{ $name }' non può essere scritto nel wrapper Windows: contiene una virgoletta doppia o un a capo.
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' non è un nome host di messaggistica nativa valido per { $browser }: deve essere composto da parti separate da punti di lettere, cifre e trattini bassi, in minuscolo per ogni browser tranne firefox.
error-browser-release-log-lock = Impossibile bloccare il registro dei rilasci al browser: { $path }
error-browser-release-log-no-data-dir = Impossibile determinare la directory dati del registro dei rilasci al browser.
error-browser-release-log-failed = Il rilascio non è stato annotato nel registro dei rilasci al browser, quindi le credenziali non sono state inviate.
error-browser-caller-not-allowed = L'estensione del browser '{ $caller }' non può usare questo host pasejo. Eseguire `pasejo browser install-manifest` per questa estensione oppure aggiungerla con --allowed-caller.
error-browser-caller-unnamed = Il browser non ha indicato quale estensione ha avviato questo host pasejo, quindi non è possibile verificarla con --allowed-caller.
browser-log-empty = Nessuna credenziale è stata ancora rilasciata a un browser.
browser-log-unreadable-lines = Righe illeggibili ignorate nel registro dei rilasci al browser: { $count }
browser-log-header-time = Ora
browser-log-header-domain = Dominio
browser-log-header-store = Archivio
browser-log-header-secret = Segreto
browser-log-header-caller = Chiamante
