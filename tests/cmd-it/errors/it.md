```
$ pasejo secret add some-secret
? 1
Error: Nessun archivio trovato nella configurazione. Eseguire prima 'pasejo store add ...' per aggiungerne uno

$ pasejo store add --path store --name primary
Archivio 'primary' aggiunto in '[CWD]/store'

$ pasejo store add --path other --name primary
? 1
Error: Il nome dell'archivio esiste già. Usare un nome diverso.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': L'archivio con nome 'missing' non esiste nella configurazione

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Valore 'tru' non valido per l'opzione 'ignore-missing-identities': atteso uno tra true, false, 1, 0, yes, no, y, n

$ pasejo store decrypt
? 1
Error: Decifratura rifiutata: stamperà ogni segreto in chiaro su stdout. Passa --yes-i-know per confermare.

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Il browser 'chrome' richiede --extension-id perché gli ID delle estensioni dei browser basati su Chromium sono derivati dalla chiave pubblica dell'estensione decompressa.

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' non è un nome host di messaggistica nativa valido per firefox: deve essere composto da parti separate da punti di lettere, cifre e trattini bassi, in minuscolo per ogni browser tranne firefox.

$ pasejo otp add --type totp --secret JBSWY3DPEHPK3PX0 not-base32
? 1
Error: Il segreto della password monouso deve essere una stringa base32 non vuota (lettere A-Z e cifre 2-7)

$ pasejo otp add --url otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PX0 not-base32-url
? 1
Error: L'URL otpauth non descrive una password monouso valida

```
