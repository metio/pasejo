```
$ pasejo secret add some-secret
? 1
Error: Kein Speicher in der Konfiguration gefunden. Führen Sie zuerst 'pasejo store add ...' aus, um einen hinzuzufügen

$ pasejo store add --path store --name primary
Speicher 'primary' bei '[CWD]/store' hinzugefügt

$ pasejo store add --path other --name primary
? 1
Error: Speichername existiert bereits. Bitte verwenden Sie einen anderen Namen.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': Speicher mit Name 'missing' existiert nicht in der Konfiguration

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Ungültiger Wert 'tru' für Option 'ignore-missing-identities': erwartet wird eines von true, false, 1, 0, yes, no, y, n

$ pasejo store decrypt
? 1
Error: Entschlüsselung verweigert: Dadurch werden alle Geheimnisse im Klartext auf stdout ausgegeben. Übergeben Sie --yes-i-know zur Bestätigung.

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: Browser 'chrome' erfordert --extension-id, da Erweiterungs-IDs für Chromium-basierte Browser aus dem öffentlichen Schlüssel der entpackten Erweiterung abgeleitet werden.

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' ist kein gültiger Native-Messaging-Hostname für firefox: Er muss aus durch Punkte getrennten Teilen aus Buchstaben, Ziffern und Unterstrichen bestehen, für jeden Browser außer firefox in Kleinbuchstaben.

$ pasejo identity remove --file non-existing
? 2
error: ungültiger Wert 'non-existing' für '--file <FILE>': Datei entspricht keiner bekannten Identität

Usage: pasejo identity remove [OPTIONS] --file <FILE>

For more information, try '--help'.

$ pasejo otp add --type totp --secret JBSWY3DPEHPK3PX0 not-base32
? 1
Error: Das Geheimnis des Einmalpassworts muss eine nicht leere Base32-Zeichenkette sein (Buchstaben A-Z und Ziffern 2-7)

```
