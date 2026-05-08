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
