# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Empfänger

recipient-added = Empfänger für '{ $public_key }' hinzugefügt
recipient-removed = Empfänger für '{ $public_key }' entfernt
recipient-does-not-exist-ignored = Empfänger für '{ $public_key }' existiert nicht im Speicher – wird ignoriert

## Geheimnisse

secret-added = Geheimnis bei '{ $secret_path }' hinzugefügt
secret-edited = Geheimnis bei '{ $secret_path }' bearbeitet
secret-generated = Geheimnis bei '{ $secret_path }' erzeugt
secret-copied = Geheimnis von '{ $source_path }' nach '{ $target_path }' kopiert
secret-moved = Geheimnis von '{ $source_path }' nach '{ $target_path }' verschoben
secret-removed = Geheimnis bei '{ $secret_path }' entfernt
secret-show-as-qrcode = Zeige Geheimnis bei '{ $secret_path }' als QR-Code
secret-show-as-text = Zeige Geheimnis bei '{ $secret_path }' als Text
secret-copy-into-clipboard = Geheimnis '{ $secret_path }' in die Zwischenablage kopiert — wird in { $duration } gelöscht (Strg-C zum sofortigen Löschen)

## Einmalpasswörter

one-time-password-added = Einmalpasswort bei '{ $password_path }' hinzugefügt
one-time-password-copied = Einmalpasswort von '{ $source_path }' nach '{ $target_path }' kopiert
one-time-password-moved = Einmalpasswort von '{ $source_path }' nach '{ $target_path }' verschoben
one-time-password-removed = Einmalpasswort bei '{ $password_path }' entfernt
one-time-password-show = Zeige Einmalpasswort bei '{ $password_path }'
one-time-password-copy-into-clipboard = Einmalpasswort '{ $password_path }' in die Zwischenablage kopiert — wird in { $duration } gelöscht (Strg-C zum sofortigen Löschen)

## Identitäten

identity-added = Identität mit Datei '{ $identity_file }' hinzugefügt
identity-removed = Identität mit Datei '{ $identity_file }' entfernt
no-identities-exist-yet = Im Speicher '{ $store_name }' sind noch keine Identitäten vorhanden. Bitte fügen Sie eine mit 'pasejo identity add ...' hinzu.

## Speicher

store-add-success = Speicher '{ $store_name }' bei '{ $store_path }' hinzugefügt
store-set-default = Speicher '{ $store_name }' ist jetzt der Standard
store-remove-success = Speicher '{ $store_name }' entfernt

## Hook-Ausführung

execute-pull-hooks = Führe Pull-Hooks für Speicher '{ $store_name }' aus
execute-push-hooks = Führe Push-Hooks für Speicher '{ $store_name }' aus

## Merge-Konflikte

merge-conflict-recipient-names = Merge-Konflikt für Empfänger mit öffentlichem Schlüssel '{ $public_key }': Namen '{ $first_name }' und '{ $second_name }' unterscheiden sich
merge-conflict-recipient-removed-and-renamed = Merge-Konflikt für Empfänger mit öffentlichem Schlüssel '{ $public_key }': Empfänger wurde in einer Version entfernt und in der anderen zu '{ $new_name }' umbenannt
merge-conflict-values = Merge-Konflikt für { $value_type } bei '{ $secret_path }': Werte unterscheiden sich in den beiden Versionen
merge-conflict-removed-and-modified = Merge-Konflikt für { $value_type } bei '{ $secret_path }': { $value_type } wurde in einer Version entfernt und in der anderen geändert

## Listenausgabe (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = Speicher: { $identity_file }
list-global-pull-hook = Global Pull: { $command }
list-global-push-hook = Global Push: { $command }
list-store-pull-hook = Speicher Pull: { $command }
list-store-push-hook = Speicher Push: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (Standard)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## Zwischenablage

clipboard-read-for-compare-failed = Konnte Zwischenablage zum Vergleich nicht lesen: { $error }
clipboard-ctrlc-handler-install-failed = Konnte Strg-C-Handler nicht installieren: { $error }. Zwischenablage wird erst nach Ablauf des konfigurierten Timeouts geleert.
clipboard-clear-failed = Konnte Zwischenablage nicht leeren: { $error }
clipboard-manual-clear-required = Zwischenablage konnte nicht automatisch geleert werden — bitte jetzt manuell leeren.
clipboard-notification-dispatch-failed = Konnte Benachrichtigung über das Leeren der Zwischenablage nicht anzeigen: { $error }
clipboard-drop-clear-failed = Konnte Zwischenablage beim Aufräumen nicht leeren: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] Zwischenablage geleert (abgebrochen)
   *[false] Zwischenablage geleert
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] Zwischenablage unverändert (Sie haben etwas anderes kopiert) (abgebrochen)
   *[false] Zwischenablage unverändert (Sie haben etwas anderes kopiert)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] Zwischenablage geleert (Inhalt konnte nicht überprüft werden) (abgebrochen)
   *[false] Zwischenablage geleert (Inhalt konnte nicht überprüft werden)
  }
clipboard-notification-failed = { $cancelled ->
    [true] Zwischenablage konnte nicht geleert werden! Bitte manuell leeren. (abgebrochen)
   *[false] Zwischenablage konnte nicht geleert werden! Bitte manuell leeren.
  }

## Eingabeaufforderungen

prompt-enter-secret = Geheimnis für { $secret_path } eingeben:
prompt-could-not-read-secret = Konnte Geheimnis für { $secret_path } nicht lesen
prompt-overwrite-secret = Bestehendes Geheimnis überschreiben?
prompt-remove-secret = Bestehendes Geheimnis entfernen?
prompt-overwrite-one-time-password = Bestehendes Einmalpasswort überschreiben?
prompt-remove-one-time-password = Bestehendes Einmalpasswort entfernen?

## Fehlermeldungen

error-cannot-get-user-confirmation = Bestätigung des Benutzers konnte nicht eingeholt werden
error-no-confirmation-from-non-terminal = Bestätigung kann nicht von einer nicht-terminalen Eingabe eingeholt werden. Verwenden Sie --force, um die Bestätigung zu überspringen.
error-could-not-load-configuration = Konfiguration konnte nicht geladen werden
error-store-does-not-exist = Speicher mit Name '{ $store_name }' existiert nicht in der Konfiguration
error-invalid-line-number = '{ $input }' ist keine gültige Zeilennummer
error-line-number-must-not-be-zero = Zeilennummer darf nicht 0 sein. Verwenden Sie 1 für die erste Zeile, -1 für die letzte
error-invalid-count = '{ $input }' ist keine gültige Anzahl
error-count-must-not-be-zero = Anzahl darf nicht 0 sein. Verwenden Sie 1, um die erste Zeile zu überspringen
error-file-does-not-exist = Die Datei '{ $input }' existiert nicht
error-secret-already-exists = Geheimnis existiert bereits bei { $secret_path }. Verwenden Sie --force, um es zu überschreiben.
error-secret-already-exists-overwrite-or-inplace = Geheimnis existiert bereits bei { $secret_path }. Verwenden Sie --force, um es vollständig zu überschreiben, oder --inplace, um nur die erste Zeile zu ändern.
error-no-secret-found = Kein Geheimnis bei '{ $secret_path }' gefunden
error-not-allowed-to-remove-secret = Entfernen des Geheimnisses bei { $secret_path } nicht erlaubt. Verwenden Sie --force zum Überschreiben.
error-secret-does-not-exist-for-edit = Geheimnis bei { $secret_path } existiert nicht. Verwenden Sie 'pasejo secret add', um es zu erstellen.
error-one-time-password-already-exists = Einmalpasswort existiert bereits bei { $password_path }. Verwenden Sie --force, um es zu überschreiben.
error-not-allowed-to-remove-one-time-password = Entfernen des Einmalpassworts bei { $password_path } nicht erlaubt. Verwenden Sie --force zum Überschreiben.
error-no-one-time-password-found = Kein Einmalpasswort bei '{ $password_path }' gefunden
error-no-store-in-configuration = Kein Speicher in der Konfiguration gefunden. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen
error-decrypt-requires-yes-i-know = Entschlüsselung verweigert: Dadurch werden alle Geheimnisse im Klartext auf stdout ausgegeben. Übergeben Sie --yes-i-know zur Bestätigung.
error-no-store-or-global = Kein Speicher in der Konfiguration gefunden und kein --global-Flag angegeben. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen
error-store-name-already-exists = Speichername existiert bereits. Bitte verwenden Sie einen anderen Namen.
error-store-path-is-directory = Verzeichnis kann nicht als Speicherpfad verwendet werden. Bitte verwenden Sie einen Dateipfad.
error-cannot-create-store-path = Speicherpfad konnte nicht erstellt werden. Bitte überprüfen Sie den Pfad und versuchen Sie es erneut.
error-cannot-get-store-parent = Übergeordnetes Verzeichnis des Speicherpfads konnte nicht ermittelt werden. Bitte überprüfen Sie den Pfad und versuchen Sie es erneut.
error-cannot-identify-store = Speicher konnte nicht identifiziert werden. Legen Sie einen Standardspeicher fest, verwenden Sie --store, um einen Speicher anzugeben, oder verwenden Sie --global, um die Identität global zu setzen.
error-no-identity-files-to-decrypt = Keine Identitätsdateien zum Entschlüsseln. Fügen Sie mindestens eine Identität hinzu, um die Speicher-Initialisierung abzuschließen.
error-invalid-ssh-public-key-format = Ungültiges Format für öffentlichen SSH-Schlüssel
error-username-empty = Benutzername darf nicht leer sein
error-username-contains-dotdot = Ungültiger Benutzername '{ $username }': darf '..' nicht enthalten
error-username-invalid-character = Ungültiger Benutzername '{ $username }': muss mit einem ASCII-Buchstaben oder einer Ziffer beginnen und darf nur ASCII-Buchstaben, Ziffern, '-', '_' und '.' enthalten
error-no-public-key-found-in-file = Kein öffentlicher Schlüssel in '{ $filename }' gefunden
error-no-public-key-source = Sie müssen mindestens eine Quelle für einen öffentlichen Schlüssel angeben
error-no-qrcode-found = Kein QR-Code in '{ $qrcode }' gefunden
error-failed-to-decode-qrcode = QR-Code in '{ $qrcode }' konnte nicht dekodiert werden
error-qr-sandbox-failed = QR-Code-Dekoder konnte nicht in der Sandbox gestartet werden
error-qr-sandbox-child-signal = QR-Code-Dekoder wurde durch Signal { $signal } beendet
error-qr-sandbox-not-enforced = QR-Code-Dekoder-Sandbox konnte nicht erzwungen werden; der laufende Kernel unterstützt kein Landlock
error-cannot-determine-store-name = Speichername konnte nicht ermittelt werden
error-cannot-parse-hook-command = Befehl konnte nicht geparst werden: { $command }
error-empty-hook-command = Leerer Hook-Befehl: { $command }
error-failed-to-run-hook = Hook { $command } konnte nicht ausgeführt werden
error-hook-failed-no-detail = Hook { $command } fehlgeschlagen (Exit { $exit })
error-hook-failed-with-detail = Hook { $command } fehlgeschlagen (Exit { $exit }): { $detail }
error-cannot-determine-store-parent-path = Übergeordneter Pfad des Speicherpfads { $path } konnte nicht ermittelt werden
error-store-path-not-utf8 = %p in Token { $token } konnte nicht ersetzt werden: Speicherpfad { $path } ist kein gültiges UTF-8
error-merge-conflict-recipients = Merge-Konflikt bei Empfängern erkannt. Bitte manuell auflösen.
error-recipient-not-found-in-store = Empfänger nicht im Speicher gefunden
error-cannot-decrypt-common-ancestor-store = Gemeinsamer Vorgängerspeicher konnte nicht entschlüsselt werden
error-cannot-decrypt-current-version-store = Aktueller Versionsspeicher konnte nicht entschlüsselt werden
error-cannot-decrypt-other-version-store = Anderer Versionsspeicher konnte nicht entschlüsselt werden
error-cannot-read-file = Datei bei '{ $path }' konnte nicht gelesen werden
error-downloading-public-key-failed = Herunterladen des öffentlichen Schlüssels von { $provider } fehlgeschlagen
error-cannot-determine-parent-directory = Übergeordnetes Verzeichnis von { $path } konnte nicht ermittelt werden
error-failed-to-create-directory = Verzeichnis { $path } konnte nicht erstellt werden
error-failed-to-write-file = { $path } konnte nicht geschrieben werden
error-failed-to-fsync-file = fsync auf { $path } fehlgeschlagen
error-failed-to-rename-file = { $from } konnte nicht in { $to } umbenannt werden
error-failed-to-create-file = { $path } konnte nicht erstellt werden
error-could-not-resolve-config-path = Absoluter Pfad zur Konfiguration konnte nicht aufgelöst werden
error-could-not-determine-config-path = Konfigurationspfad konnte nicht ermittelt werden
error-could-not-resolve-store-path = Absoluter Speicherpfad konnte nicht aufgelöst werden
error-could-not-create-config-dir = Konfigurationsverzeichnis konnte nicht erstellt werden
error-could-not-migrate-legacy-config = Legacy-Konfigurationsdatei konnte nicht migriert werden
error-could-not-move-file = Datei konnte nicht verschoben werden
error-could-not-copy-file = Datei konnte nicht kopiert werden
error-could-not-remove-source-after-copy = Quelldatei konnte nach dem Kopieren nicht entfernt werden
error-could-not-read-configuration = Konfiguration konnte nicht gelesen werden
error-could-not-serialize-migrated-config = Migrierte Konfiguration konnte nicht serialisiert werden
error-could-not-store-configuration = Konfiguration konnte nicht gespeichert werden
error-could-not-load-migrated-config = Migrierte Konfiguration konnte nicht geladen werden
error-could-not-serialize-configuration = Konfiguration konnte nicht serialisiert werden
error-config-not-valid-toml = Konfigurationsdatei bei { $path } ist kein gültiges TOML
error-could-not-open-store-for-lock = Speicherdatei zum Sperren konnte nicht geöffnet werden: { $path }
error-could-not-acquire-store-lock = Sperre auf Speicherdatei konnte nicht erworben werden: { $path }
error-cannot-encrypt-store = Speicher konnte nicht verschlüsselt werden
error-failed-to-run-command = Befehl { $binary } konnte nicht ausgeführt werden
error-command-exited-with = Befehl { $binary } beendet mit { $exit }
