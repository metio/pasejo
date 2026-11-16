# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Empfänger

recipient-added = Empfänger für '{ $public_key }' hinzugefügt
recipient-removed = Empfänger für '{ $public_key }' entfernt
recipient-does-not-exist-ignored = Empfänger für '{ $public_key }' existiert nicht im Speicher – wird ignoriert
recipient-unsupported-key-skipped = { $provider }-Schlüssel vom Typ '{ $key_type }' für Benutzer '{ $username }' wird übersprungen: age kann nicht an ihn verschlüsseln

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
store-remove-success-with-data = Speicher '{ $store_name }' entfernt und seine verschlüsselte Datendatei gelöscht

## Hook-Ausführung

execute-pull-hooks = Führe Pull-Hooks für Speicher '{ $store_name }' aus
execute-push-hooks = Führe Push-Hooks für Speicher '{ $store_name }' aus
hook-run-nothing-selected = Nichts auszuführen: Geben Sie --pull und/oder --push an, um auszuwählen, welche Hooks ausgeführt werden sollen.
export-plaintext-warning = Warnung: Der Export enthält jedes Geheimnis im Klartext.
export-hotp-omitted = Warnung: Bitwarden kann keine HOTP-Einmalpasswörter speichern, daher sind diese nicht Teil des Exports: { $names }
export-orphaned-otp-omitted = Warnung: Diese Einmalpasswörter haben kein gleichnamiges Geheimnis und sind nicht Teil des Exports: { $names }
export-written-to-file = Export nach '{ $path }' geschrieben

## Merge-Konflikte

merge-conflict-recipient-names = Merge-Konflikt für Empfänger mit öffentlichem Schlüssel '{ $public_key }': Namen '{ $first_name }' und '{ $second_name }' unterscheiden sich
merge-conflict-recipient-removed-and-renamed = Merge-Konflikt für Empfänger mit öffentlichem Schlüssel '{ $public_key }': Empfänger wurde in einer Version entfernt und in der anderen zu '{ $new_name }' umbenannt
merge-conflict-values = Merge-Konflikt für { $value_type } bei '{ $secret_path }': Werte unterscheiden sich in den beiden Versionen
merge-conflict-removed-and-modified = Merge-Konflikt für { $value_type } bei '{ $secret_path }': { $value_type } wurde in einer Version entfernt und in der anderen geändert

## Listenausgabe (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = Speicher: { $identity_file }
list-store-identity-also-global = Speicher: { $identity_file } (auch global registriert)
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
error-editor-launch-failed = Konnte den Editor '{ $editor }' nicht starten
error-editor-exited-unsuccessfully = Der Editor '{ $editor }' wurde nicht erfolgreich beendet, daher bleibt das Geheimnis unverändert
error-editor-file-not-utf8 = Das bearbeitete Geheimnis ist kein gültiges UTF-8
error-editor-file-create-failed = Konnte keine temporäre Datei für den Editor in '{ $path }' anlegen
editor-file-in-temp-dir = Weder $XDG_RUNTIME_DIR noch /dev/shm ist verfügbar, daher wird das Geheimnis während der Bearbeitung in eine Datei in '{ $path }' geschrieben. Dieses Verzeichnis liegt möglicherweise auf der Festplatte.
editor-file-cleanup-failed = Konnte die temporäre Editor-Datei '{ $path }' nicht überschreiben und entfernen: { $error }
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
error-password-length-zero = Die Passwortlänge darf nicht 0 sein
error-password-length-too-short = Die Passwortlänge ist zu kurz für die geforderten Zeichenarten
error-password-no-character-kinds = Mindestens eine Zeichenart muss aktiviert sein
error-password-generation-failed = Passwort kann nicht erzeugt werden: { $reason }
error-no-secret-found = Kein Geheimnis bei '{ $secret_path }' gefunden
error-not-allowed-to-remove-secret = Entfernen des Geheimnisses bei { $secret_path } nicht erlaubt. Verwenden Sie --force zum Überschreiben.
error-secret-does-not-exist-for-edit = Geheimnis bei { $secret_path } existiert nicht. Verwenden Sie 'pasejo secret add', um es zu erstellen.
error-one-time-password-already-exists = Einmalpasswort existiert bereits bei { $password_path }. Verwenden Sie --force, um es zu überschreiben.
error-not-allowed-to-remove-one-time-password = Entfernen des Einmalpassworts bei { $password_path } nicht erlaubt. Verwenden Sie --force zum Überschreiben.
error-no-one-time-password-found = Kein Einmalpasswort bei '{ $password_path }' gefunden
error-invalid-one-time-password-secret = Das Geheimnis des Einmalpassworts muss eine nicht leere Base32-Zeichenkette sein (Buchstaben A-Z und Ziffern 2-7)
error-invalid-otpauth-url = Die otpauth-URL beschreibt kein gültiges Einmalpasswort
error-hotp-counter-exhausted = Der HOTP-Zähler hat seinen Höchstwert erreicht und kann nicht weiter erhöht werden
error-no-store-in-configuration = Kein Speicher in der Konfiguration gefunden. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen
error-unknown-store = Speicher '{ $store_name }' existiert nicht in der Konfiguration. Führen Sie 'pasejo store list' aus, um die konfigurierten Speicher anzuzeigen.
error-decrypt-requires-yes-i-know = Entschlüsselung verweigert: Dadurch werden alle Geheimnisse im Klartext auf stdout ausgegeben. Übergeben Sie --yes-i-know zur Bestätigung.
error-no-store-or-global = Kein Speicher in der Konfiguration gefunden und kein --global-Flag angegeben. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen
error-store-name-already-exists = Speichername existiert bereits. Bitte verwenden Sie einen anderen Namen.
error-store-path-is-directory = Verzeichnis kann nicht als Speicherpfad verwendet werden. Bitte verwenden Sie einen Dateipfad.
error-cannot-create-store-path = Speicherpfad konnte nicht erstellt werden. Bitte überprüfen Sie den Pfad und versuchen Sie es erneut.
error-cannot-get-store-parent = Übergeordnetes Verzeichnis des Speicherpfads konnte nicht ermittelt werden. Bitte überprüfen Sie den Pfad und versuchen Sie es erneut.
error-cannot-identify-store = Speicher konnte nicht identifiziert werden. Legen Sie einen Standardspeicher fest, verwenden Sie --store, um einen Speicher anzugeben, oder verwenden Sie --global, um die Identität global zu setzen.
error-identity-file-already-added = ungültiger Wert '{ $file }' für '--file <FILE>': Datei wurde bereits als Identität hinzugefügt
error-identity-file-not-known = ungültiger Wert '{ $file }' für '--file <FILE>': Datei entspricht keiner bekannten Identität
error-identity-did-you-mean = Meinten Sie '{ $stored }'? Identitätspfade werden exakt verglichen.
error-no-identity-files-to-decrypt = Keine Identitätsdateien zum Entschlüsseln. Fügen Sie mindestens eine Identität hinzu, um die Speicher-Initialisierung abzuschließen.
error-invalid-ssh-public-key-format = Ungültiges Format für öffentlichen SSH-Schlüssel
error-username-empty = Benutzername darf nicht leer sein
error-username-contains-dotdot = Ungültiger Benutzername '{ $username }': darf '..' nicht enthalten
error-username-invalid-character = Ungültiger Benutzername '{ $username }': muss mit einem ASCII-Buchstaben oder einer Ziffer beginnen und darf nur ASCII-Buchstaben, Ziffern, '-', '_' und '.' enthalten
error-no-public-key-found-in-file = Kein öffentlicher Schlüssel in '{ $filename }' gefunden
error-no-public-keys-for-user = { $provider }-Benutzer '{ $username }' hat keine öffentlichen SSH-Schlüssel, die als Empfänger hinzugefügt werden könnten
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
error-merge-conflict = Merge-Konflikt bei { $value_type }s erkannt. Bitte manuell auflösen.
error-recipient-not-found-in-store = Empfänger nicht im Speicher gefunden
error-cannot-remove-last-recipient = Der letzte Empfänger kann nicht aus dem Speicher entfernt werden. Bitte fügen Sie einen neuen Empfänger hinzu, bevor Sie diesen entfernen.
error-cannot-decrypt-common-ancestor-store = Gemeinsamer Vorgängerspeicher konnte nicht entschlüsselt werden
error-cannot-decrypt-current-version-store = Aktueller Versionsspeicher konnte nicht entschlüsselt werden
error-cannot-decrypt-other-version-store = Anderer Versionsspeicher konnte nicht entschlüsselt werden
error-cannot-read-file = Datei bei '{ $path }' konnte nicht gelesen werden
error-downloading-public-key-failed = Herunterladen des öffentlichen Schlüssels von { $provider } fehlgeschlagen
error-download-user-not-found = { $provider } kennt keinen Benutzer '{ $username }'
error-download-rate-limited = { $provider } hat die Anfrage gedrosselt; bitte einen Moment warten und erneut versuchen
error-download-server-error = { $provider } hat einen Fehler zurückgegeben (HTTP { $status })
error-download-timeout = Zeitüberschreitung beim Herunterladen des öffentlichen Schlüssels von { $provider }; Verbindung prüfen oder key-download-timeout-seconds erhöhen
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
error-config-invalid-integer = Ungültiger Wert '{ $value }' für Option '{ $option }': eine nicht-negative Ganzzahl wird erwartet
error-config-invalid-boolean = Ungültiger Wert '{ $value }' für Option '{ $option }': erwartet wird eines von true, false, 1, 0, yes, no, y, n
error-could-not-open-store-for-lock = Speicherdatei zum Sperren konnte nicht geöffnet werden: { $path }
error-could-not-acquire-store-lock = Sperre auf Speicherdatei konnte nicht erworben werden: { $path }
error-store-lock-wait-timed-out = Zeitüberschreitung beim Warten auf die Sperre der Speicherdatei: { $path }
error-cannot-encrypt-store = Speicher konnte nicht verschlüsselt werden
error-failed-to-run-command = Befehl { $binary } konnte nicht ausgeführt werden
error-command-exited-with = Befehl { $binary } beendet mit { $exit }
error-no-field-found = Das Geheimnis '{ $secret_path }' hat kein Feld '{ $field }'
error-no-line-found = Das Geheimnis '{ $secret_path }' hat keine Zeile { $line }
error-value-unresolved = Der Wert von '{ $name }' kann nicht ermittelt werden
error-env-assignment-invalid = '{ $input }' hat nicht die Form VARIABLE=GEHEIMNIS
error-env-field-assignment-invalid = '{ $input }' hat nicht die Form VARIABLE=GEHEIMNIS=FELD
error-env-variable-name-invalid = '{ $variable }' ist kein gültiger Name für eine Umgebungsvariable. Verwenden Sie Buchstaben, Ziffern und Unterstriche, und beginnen Sie nicht mit einer Ziffer
error-env-variable-duplicate = Die Umgebungsvariable { $variable } wird mehr als einmal zugewiesen
error-env-reference-line-and-field = Der Verweis für { $variable } setzt sowohl 'line' als auch 'field'
error-resolve-input-invalid = Die Verweise auf Geheimnisse können nicht von der Standardeingabe gelesen werden
exec-value-too-short-to-redact = Der Wert von { $variable } ist kürzer als { $length } Zeichen und wird nicht geschwärzt
error-browser-host-requires-pipe = Der Browser-Host benötigt eine weitergeleitete Standardeingabe von der verbundenen Browser-Erweiterung. Führen Sie diesen Befehl über das Native-Messaging-Manifest Ihres Browsers aus, nicht direkt in einem Terminal.
error-browser-store-file-missing = Speicherdatei existiert nicht: { $path }
error-browser-identity-passphrase-protected = Der Browser-Host kann nicht nach der Passphrase einer verschlüsselten Identität fragen. Verwenden Sie für diesen Speicher eine unverschlüsselte oder Plugin-gestützte Identität.
prompt-browser-host-pinentry = PIN oder Passphrase
error-browser-message-too-large = Nachricht mit { $length } Bytes überschreitet die Native-Messaging-Grenze von { $limit } Bytes
error-browser-response-too-large = Antwort mit { $length } Bytes überschreitet die Native-Messaging-Grenze von { $limit } Bytes
error-browser-install-manifest-no-browser = Geben Sie mindestens einen --browser an, für den das Native-Messaging-Manifest installiert werden soll.
error-browser-install-manifest-needs-extension-id = Browser '{ $browser }' erfordert --extension-id, da Erweiterungs-IDs für Chromium-basierte Browser aus dem öffentlichen Schlüssel der entpackten Erweiterung abgeleitet werden.
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id kann nicht mit mehr als einem --browser kombiniert werden, da jeder Browser seine eigene Erweiterungs-ID benötigt. Führen Sie den Befehl pro Browser einmal aus.
error-browser-install-manifest-unsupported-os = Die Installation von Native-Messaging-Manifesten wird unter '{ $os }' nicht unterstützt.
error-browser-install-manifest-registry-failed = Der Native-Messaging-Registrierungsschlüssel '{ $key }' konnte nicht geschrieben werden
error-browser-install-manifest-no-home-dir = Das Home-Verzeichnis des Benutzers konnte nicht ermittelt werden.
error-browser-install-manifest-no-data-dir = Das Daten-Verzeichnis des Benutzers konnte nicht ermittelt werden.
error-browser-install-manifest-no-current-exe = Der Pfad der laufenden pasejo-Binärdatei konnte nicht ermittelt werden.
browser-install-manifest-written = Native-Messaging-Manifest für '{ $browser }' nach '{ $path }' geschrieben
browser-install-manifest-print-wrapper-note = Hinweis: Das ausgegebene Manifest verweist auf das Wrapper-Skript unter '{ $path }', das dieser Befehl nicht installiert hat. Verwenden Sie --binary, um stattdessen auf eine vorhandene ausführbare Datei zu verweisen.
error-browser-install-manifest-invalid-env = --env-Eintrag '{ $entry }' hat nicht die Form KEY=VALUE.
error-browser-install-manifest-invalid-env-name = --env-Name '{ $name }' ist kein gültiger POSIX-Umgebungsvariablenname (muss mit einem Buchstaben oder Unterstrich beginnen und darf nur Buchstaben, Ziffern und Unterstriche enthalten).
error-browser-install-manifest-invalid-env-value-windows = Der --env-Wert von '{ $name }' kann nicht in den Windows-Wrapper geschrieben werden: Er enthält ein doppeltes Anführungszeichen oder einen Zeilenumbruch.
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' ist kein gültiger Native-Messaging-Hostname für { $browser }: Er muss aus durch Punkte getrennten Teilen aus Buchstaben, Ziffern und Unterstrichen bestehen, für jeden Browser außer firefox in Kleinbuchstaben.
error-browser-install-manifest-invalid-extension-id = --extension-id '{ $id }' ist keine gültige Erweiterungs-ID: Sie darf nur ASCII-Buchstaben, Ziffern, '@', '.', '_', '-' und geschweifte Klammern enthalten.
error-browser-release-log-lock = Das Browser-Freigabeprotokoll konnte nicht gesperrt werden: { $path }
error-browser-release-log-no-data-dir = Das Daten-Verzeichnis für das Browser-Freigabeprotokoll konnte nicht ermittelt werden.
error-browser-release-log-failed = Die Freigabe konnte nicht im Browser-Freigabeprotokoll vermerkt werden, daher wurden die Zugangsdaten nicht gesendet.
error-browser-caller-not-allowed = Die Browser-Erweiterung '{ $caller }' darf diesen pasejo-Host nicht verwenden. Führen Sie `pasejo browser install-manifest` für diese Erweiterung aus oder fügen Sie sie mit --allowed-caller hinzu.
error-browser-caller-unnamed = Der Browser hat die Erweiterung, die diesen pasejo-Host gestartet hat, nicht benannt, daher kann sie nicht mit --allowed-caller abgeglichen werden.
browser-log-empty = Bisher wurden keine Zugangsdaten an einen Browser freigegeben.
browser-log-unreadable-lines = Übersprungene unlesbare Zeilen im Browser-Freigabeprotokoll: { $count }
browser-log-header-time = Zeit
browser-log-header-domain = Domain
browser-log-header-store = Speicher
browser-log-header-secret = Geheimnis
browser-log-header-caller = Aufrufer

## Remote stores
error-invalid-date = '{ $value }' ist kein Datum im Format JJJJ-MM-TT.
error-remote-actions-failed = Einige ausstehende Aktionen sind fehlgeschlagen.
error-remote-bad-request = Der Server hat die Anfrage nicht angenommen.
error-remote-checkpoint-key-changed = Der Checkpoint-Schlüssel von { $url } unterscheidet sich von dem bei der ersten Anmeldung gespeicherten. Der Server wurde möglicherweise ersetzt; fragen Sie seine Betreiber, bevor Sie sich erneut anmelden.
error-remote-concurrent-change = Jemand anderes hat '{ $name }' gleichzeitig geändert. Führen Sie den Befehl erneut aus, um Ihre Änderung auf die neue Version anzuwenden.
error-remote-conflict = Der Server hat eine neuere Version. Führen Sie den Befehl erneut aus.
error-remote-device-user-not-member = { $email } ist kein Mitglied der Organisation. Fügen Sie die Person zuerst mit 'pasejo org role { $email } member' hinzu.
error-remote-fingerprint-mismatch = Der Fingerabdruck stimmt nicht überein. Der Server meldet { $expected }.
error-remote-fingerprint-not-confirmed = Der Fingerabdruck wurde nicht bestätigt.
error-remote-fingerprint-required = Vergleichen Sie den Fingerabdruck { $expected } über einen anderen Kanal und übergeben Sie ihn dann mit --fingerprint.
error-remote-forbidden = Der Server hat abgelehnt: Ihnen fehlt die Berechtigung dafür.
error-remote-invalid-id = '{ $value }' ist keine gültige Kennung.
error-remote-invalid-recipient = '{ $recipient }' ist kein age-Empfänger, an den der Server verschlüsseln kann.
error-remote-login-denied = Die Anmeldung wurde im Browser abgelehnt.
error-remote-login-expired = Die Anmeldung wurde nicht rechtzeitig bestätigt. Führen Sie 'pasejo server login' erneut aus.
error-remote-needs-second-admin = Diese Änderung benötigt die Signatur eines zweiten Admins.
error-remote-no-data-dir = Es gibt kein lokales Datenverzeichnis für die Schlüssel dieses Geräts. Setzen Sie PASEJO_DATA_DIR.
error-remote-no-org = Es ist keine Organisation registriert. Führen Sie 'pasejo org create' oder 'pasejo org join' aus.
error-remote-no-recovery-recipient = Die Organisation { $org } hat keinen Wiederherstellungsempfänger.
error-remote-no-server = Es ist kein Server registriert. Führen Sie 'pasejo server login URL' aus.
error-remote-no-session = Dieses Gerät ist nicht angemeldet.
error-remote-not-found = Der Server kennt das Angefragte nicht.
error-remote-not-logged-in = Dieses Gerät ist nicht bei { $url } angemeldet. Führen Sie 'pasejo server login { $url }' aus.
error-remote-org-ambiguous = Mehrere Organisationen passen zu '{ $org }'. Übergeben Sie die Kennung der Organisation.
error-remote-org-required = Mehrere Organisationen sind registriert. Wählen Sie eine mit --org.
error-remote-pending-device = Dieses Gerät wartet auf die Freigabe durch einen Admin oder Freigebenden der Organisation.
error-remote-rate-limited = Der Server bittet, vor einem erneuten Versuch zu warten.
error-remote-recipient-required = Keine globale Identität liefert einen age-Empfänger für dieses Gerät. Übergeben Sie einen mit --recipient.
error-remote-rejected = Das Mitgliedschaftsprotokoll der Organisation hat die Änderung abgelehnt.
error-remote-server-ambiguous = Mehrere Server sind registriert. Geben Sie die URL des Servers an.
error-remote-server-failed = Der Server konnte die Anfrage nicht beantworten.
error-remote-session-rejected = Der Server akzeptiert die Sitzung dieses Geräts nicht mehr. Führen Sie 'pasejo server login' erneut aus.
error-remote-signing-key-unreadable = Der Signaturschlüssel des Geräts unter { $path } kann nicht gelesen werden.
error-remote-store-command-not-applicable = Der Speicher '{ $store_name }' ist ein entfernter Speicher; Empfänger, Hooks und Zusammenführungen gelten für ihn nicht.
error-remote-store-name-taken = Ein Speicher namens '{ $name }' existiert bereits.
error-remote-store-required = Der Speicher '{ $store_name }' ist kein entfernter Speicher.
error-remote-team-exists = Ein Team namens '{ $name }' existiert bereits.
error-remote-too-large = Die Anfrage ist größer, als der Server annimmt.
error-remote-unavailable = Diesem Gerät fehlt, was die Anfrage benötigt.
error-remote-unknown-device = Kein solches Gerät: { $device }.
error-remote-unknown-org = Keine registrierte Organisation passt zu '{ $org }'.
error-remote-unknown-proposal = Kein offener Vorschlag hat die Kennung { $proposal }.
error-remote-unknown-server = Unter { $url } ist kein Server registriert. Führen Sie 'pasejo server login { $url }' aus.
error-remote-unknown-store = Die Organisation hat keinen Speicher { $store }.
error-remote-unknown-team = Die Organisation hat kein Team namens '{ $name }'.
error-remote-unknown-user = Niemand in der Organisation verwendet die E-Mail-Adresse { $email }.
error-remote-unreachable = Der Server ist nicht erreichbar.
error-remote-verification-failed = Was der Server zurückgegeben hat, lässt sich nicht verifizieren. Vertrauen Sie diesem Server nicht, bis seine Betreiber den Grund erklären.
error-remote-workload-token-unreadable = Das Workload-Token unter { $path } kann nicht gelesen werden.
error-remote-wrong-object-kind = Der Eintrag '{ $name }' enthält Daten unerwarteter Art.
prompt-remote-confirm-fingerprint = Stimmt der Fingerabdruck { $fingerprint } mit dem auf der anderen Seite angezeigten überein?
remote-action-approve-device = Gerät '{ $label }' von { $email } freigeben (Fingerabdruck { $fingerprint })
remote-action-cosign-checkpoint = Audit-Checkpoint { $seq } mitsignieren
remote-action-join-member = { $email } als Mitglied hinzufügen
remote-action-remove-member = { $user } aus der Organisation entfernen
remote-action-revoke-grant = Berechtigung von { $principal } für Speicher { $store } entziehen
remote-action-rotate-epoch = Schlüssel von Speicher { $store } erneuern
remote-action-sync-team = Mitglieder von Team { $team } aktualisieren
remote-action-wrap-keys = { $count } Geräten die Schlüssel von Speicher { $store } geben
remote-device-state-active = aktiv
remote-device-state-pending = ausstehend
remote-action-done = erledigt: { $action }
remote-action-skipped = übersprungen: { $action } ({ $reason })
remote-action-failed = fehlgeschlagen: { $action }: { $error }
remote-action-pending = ausstehend: { $action }
remote-audit-line = { $seq }  { $time }  { $actor }  { $action }  { $outcome }  { $store }  { $detail }
remote-device-line = { $id }  { $label }  { $email }  { $fingerprint }  { $state }
remote-proposal-line = { $proposal }  { $kind }  { $author }
remote-store-line = { $store }  { $label }  { $role }  { $local }
remote-rotation-item = { $name }
remote-status-server = { $url }: Gerät { $device } ({ $label }), Fingerabdruck { $fingerprint }, { $session ->
    [yes] angemeldet
   *[other] nicht angemeldet
}
remote-status-org = - Organisation { $name } ({ $org }), Fingerabdruck { $fingerprint }
remote-audit-verified = { $events } Audit-Ereignisse und { $checkpoints } Checkpoints verifiziert.
remote-change-proposed = Die Änderung { $kind } benötigt einen zweiten Admin. Vorschlag { $proposal } wartet auf 'pasejo org cosign'.
remote-change-recorded = { $kind } im Mitgliedschaftsprotokoll vermerkt.
remote-device-active = Dieses Gerät ist in { $org } aktiv.
remote-device-pending = Dieses Gerät wartet auf Freigabe in { $org }. Geben Sie einem Admin oder Freigebenden seine Kennung { $device } und den Fingerabdruck { $fingerprint }.
remote-device-fingerprint = Der Fingerabdruck dieses Geräts ist { $fingerprint }.
remote-logged-in = Bei { $url } als { $email } angemeldet.
remote-logged-out = Von { $url } abgemeldet.
remote-login-visit = Öffnen Sie { $url } in einem Browser und bestätigen Sie den Code { $code }.
remote-no-actions = Keine ausstehenden Aktionen.
remote-no-proposals = Keine offenen Vorschläge.
remote-no-servers = Es ist kein Server registriert.
remote-no-stores = Die Organisation hat keinen Speicher, den dieses Gerät sieht.
remote-org-created = Organisation { $name } mit der Kennung { $org } und dem Fingerabdruck { $fingerprint } erstellt.
remote-rotation-access-continues = { $user } hat weiterhin Zugriff auf den Speicher.
remote-rotation-nothing = { $user } hat nichts abgerufen, das noch erneuert werden muss.
remote-settings-unchanged = Die Einstellungen haben bereits diese Werte.
remote-automatic-approval-warning = Mit automatischer Freigabe fügt `pasejo org actions --run` jedes Gerät hinzu, das ein Mitglied der Organisation anmeldet, ohne dass jemand seinen Fingerabdruck vergleicht. Ein kompromittierter Server oder Identitätsanbieter kann dann ein eigenes Gerät anmelden und Store-Schlüssel erhalten. Prüfe `pasejo org devices` regelmäßig.
remote-offline-reads-warning = Offline-Lesezugriffe erlauben Geräten, eine Kopie der Geheimnisse zu behalten und sie ohne Server zu lesen. Ein solcher Zugriff erscheint im Audit-Log erst, wenn das Gerät ihn nach dem erneuten Verbinden meldet; ein verlorenes oder kompromittiertes Gerät meldet nichts, und das Entfernen eines Mitglieds lässt die Kopien auf seinen Geräten bestehen. Erlaube sie nur, wo das Lesen ohne Netzwerk wichtiger ist als ein vollständiges Audit-Log.
remote-audit-offline-reads-open = Offline-Lesezugriffe sind seit { $from } erlaubt. Zugriffe auf Offline-Kopien erscheinen in diesem Log nur, soweit Geräte sie gemeldet haben.
remote-audit-offline-reads-closed = Offline-Lesezugriffe waren von { $from } bis { $until } erlaubt. Zugriffe auf Offline-Kopien in dieser Zeit erscheinen in diesem Log nur, soweit Geräte sie gemeldet haben.
remote-device-approved-automatically = Das Gerät '{ $label }' wurde freigegeben, ohne dass jemand seinen Fingerabdruck verglichen hat, weil die Organisation Geräte automatisch freigibt.
remote-store-added = Entfernten Speicher { $label } als '{ $name }' registriert.
remote-store-created = Entfernten Speicher '{ $name }' mit der Kennung { $store } erstellt.
remote-store-migrated = { $count } Einträge von '{ $name }' auf den Server kopiert. Der Speicher verweist jetzt auf die entfernte Kopie; die Datei { $path } ist unverändert.
remote-store-recovered = { $count } Einträge von { $label } nach { $path } wiederhergestellt.
remote-store-rekeyed = { $count } Einträge mit den aktuellen Schlüsseln neu geschrieben.

## Remote administration
error-remote-emergency-unregistered = Die Notfall-Anmeldung braucht ein Gerät, das bereits bei { $url } registriert ist.
error-remote-unknown-review = Keine Zugriffsprüfung hat die Kennung { $review }.
error-remote-not-under-review = { $principal } hat keine Berechtigung in der Prüfung { $review }.
remote-emergency-logged-in = Über das Notfallfenster bei { $url } als { $email } angemeldet, bis { $expires }.
remote-scim-token-created = SCIM-Token { $id } erstellt. Es wird unten einmal ausgegeben und kann nicht erneut angezeigt werden.
remote-scim-token-line = { $id }  { $created }
remote-no-scim-tokens = Keine gültigen SCIM-Token.
remote-scim-token-revoked = SCIM-Token { $id } widerrufen.
remote-review-opened = Zugriffsprüfung { $review } für den Speicher { $store } eröffnet, fällig am { $due }.
remote-review-line = { $review }  { $store }  { $due }  { $state }
remote-review-item = - { $principal }  { $role }  { $decision }
remote-review-state-open = offen
remote-review-state-complete = abgeschlossen
remote-review-decision-keep = behalten
remote-review-decision-revoke = entziehen
remote-review-decision-pending = ausstehend
remote-no-reviews = Keine Zugriffsprüfungen.
remote-review-decided = '{ $decision }' für { $principal } in der Prüfung { $review } festgehalten.
remote-review-revokes-pending = Entzugsentscheidungen werden wirksam, sobald ein Admin der Organisation oder des Speichers 'pasejo org actions --run' ausführt.
