```
$ pasejo secret add some-secret
? 1
Error: No se encontró ningún almacén en la configuración. Ejecute 'pasejo store add ...' primero para añadir uno

$ pasejo store add --path store --name primary
Almacén 'primary' añadido en '[CWD]/store'

$ pasejo store add --path other --name primary
? 1
Error: El nombre del almacén ya existe. Por favor use un nombre diferente.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': El almacén con nombre 'missing' no existe en la configuración

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Valor 'tru' no válido para la opción 'ignore-missing-identities': se esperaba uno de true, false, 1, 0, yes, no, y, n

$ pasejo store decrypt
? 1
Error: Descifrado rechazado: esto imprimirá todos los secretos en texto plano en stdout. Pase --yes-i-know para confirmar.

$ pasejo browser install-manifest --browser chrome --print --binary /usr/local/bin/pasejo
? 1
Error: El navegador 'chrome' requiere --extension-id porque los ID de extensión de los navegadores basados en Chromium se derivan de la clave pública de la extensión descomprimida.

$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' no es un nombre de host de mensajería nativa válido para firefox: debe estar formado por partes separadas por puntos de letras, dígitos y guiones bajos, en minúsculas para todos los navegadores excepto firefox.

$ pasejo otp add --type totp --secret JBSWY3DPEHPK3PX0 not-base32
? 1
Error: El secreto de la contraseña de un solo uso debe ser una cadena base32 no vacía (letras A-Z y dígitos 2-7)

```
