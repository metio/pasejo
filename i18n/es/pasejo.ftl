# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinatarios

recipient-added = Destinatario para '{ $public_key }' añadido
recipient-removed = Destinatario para '{ $public_key }' eliminado
recipient-does-not-exist-ignored = El destinatario para '{ $public_key }' no existe en el almacén – se omite
recipient-unsupported-key-skipped = Se omite la clave de { $provider } de tipo '{ $key_type }' del usuario '{ $username }': age no puede cifrar para ella

## Secretos

secret-added = Secreto añadido en '{ $secret_path }'
secret-edited = Secreto editado en '{ $secret_path }'
secret-generated = Secreto generado en '{ $secret_path }'
secret-copied = Secreto copiado de '{ $source_path }' a '{ $target_path }'
secret-moved = Secreto movido de '{ $source_path }' a '{ $target_path }'
secret-removed = Secreto eliminado en '{ $secret_path }'
secret-show-as-qrcode = Mostrando secreto en '{ $secret_path }' como código QR
secret-show-as-text = Mostrando secreto en '{ $secret_path }' como texto
secret-copy-into-clipboard = Secreto '{ $secret_path }' copiado al portapapeles — se borrará en { $duration } (Ctrl-C para borrar ahora)

## Contraseñas de un solo uso

one-time-password-added = Contraseña de un solo uso añadida en '{ $password_path }'
one-time-password-copied = Contraseña de un solo uso copiada de '{ $source_path }' a '{ $target_path }'
one-time-password-moved = Contraseña de un solo uso movida de '{ $source_path }' a '{ $target_path }'
one-time-password-removed = Contraseña de un solo uso eliminada en '{ $password_path }'
one-time-password-show = Mostrando contraseña de un solo uso en '{ $password_path }'
one-time-password-copy-into-clipboard = Contraseña de un solo uso '{ $password_path }' copiada al portapapeles — se borrará en { $duration } (Ctrl-C para borrar ahora)

## Identidades

identity-added = Identidad con archivo '{ $identity_file }' añadida
identity-removed = Identidad con archivo '{ $identity_file }' eliminada
no-identities-exist-yet = Aún no hay identidades en el almacén '{ $store_name }'. Por favor, añada una con 'pasejo identity add ...'.

## Almacenes

store-add-success = Almacén '{ $store_name }' añadido en '{ $store_path }'
store-set-default = El almacén '{ $store_name }' ahora es el predeterminado
store-remove-success = Almacén '{ $store_name }' eliminado
store-remove-success-with-data = Almacén '{ $store_name }' eliminado y su archivo de datos cifrado borrado

## Ejecución de hooks

execute-pull-hooks = Ejecutando hooks de pull para el almacén '{ $store_name }'
execute-push-hooks = Ejecutando hooks de push para el almacén '{ $store_name }'
hook-run-nothing-selected = Nada que ejecutar: use --pull o --push para seleccionar qué hooks ejecutar.
export-plaintext-warning = Advertencia: la exportación contiene todos los secretos en texto plano.
export-hotp-omitted = Advertencia: Bitwarden no puede almacenar contraseñas de un solo uso HOTP, por lo que estas no forman parte de la exportación: { $names }
export-orphaned-otp-omitted = Advertencia: estas contraseñas de un solo uso no tienen un secreto con el mismo nombre y no forman parte de la exportación: { $names }
export-written-to-file = Exportación escrita en '{ $path }'

## Conflictos de fusión

merge-conflict-recipient-names = Conflicto de fusión para destinatario con clave pública '{ $public_key }': los nombres '{ $first_name }' y '{ $second_name }' difieren
merge-conflict-recipient-removed-and-renamed = Conflicto de fusión para destinatario con clave pública '{ $public_key }': el destinatario fue eliminado en una versión y renombrado a '{ $new_name }' en la otra
merge-conflict-values = Conflicto de fusión para { $value_type } en '{ $secret_path }': los valores difieren entre las dos versiones
merge-conflict-removed-and-modified = Conflicto de fusión para { $value_type } en '{ $secret_path }': { $value_type } fue eliminado en una versión y modificado en la otra

## Salida de listas (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = almacén: { $identity_file }
list-store-identity-also-global = almacén: { $identity_file } (también registrada globalmente)
list-global-pull-hook = pull global: { $command }
list-global-push-hook = push global: { $command }
list-store-pull-hook = pull almacén: { $command }
list-store-push-hook = push almacén: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path } (predeterminado)
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## Portapapeles

clipboard-read-for-compare-failed = No se pudo leer el portapapeles para comparar: { $error }
clipboard-ctrlc-handler-install-failed = No se pudo instalar el manejador de Ctrl-C: { $error }. El portapapeles solo se borrará tras el tiempo de espera configurado.
clipboard-clear-failed = No se pudo borrar el portapapeles: { $error }
clipboard-manual-clear-required = El portapapeles no se pudo borrar automáticamente — por favor, bórrelo manualmente ahora.
clipboard-notification-dispatch-failed = No se pudo mostrar la notificación de borrado del portapapeles: { $error }
clipboard-drop-clear-failed = No se pudo borrar el portapapeles durante la limpieza: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] Portapapeles borrado (cancelado)
   *[false] Portapapeles borrado
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] Portapapeles intacto (copió otra cosa) (cancelado)
   *[false] Portapapeles intacto (copió otra cosa)
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] Portapapeles borrado (no se pudo verificar el contenido) (cancelado)
   *[false] Portapapeles borrado (no se pudo verificar el contenido)
  }
clipboard-notification-failed = { $cancelled ->
    [true] ¡No se pudo borrar el portapapeles! Por favor, bórrelo manualmente. (cancelado)
   *[false] ¡No se pudo borrar el portapapeles! Por favor, bórrelo manualmente.
  }

## Indicaciones

prompt-enter-secret = Introducir secreto para { $secret_path }:
prompt-could-not-read-secret = No se pudo leer el secreto para { $secret_path }
error-editor-launch-failed = No se pudo iniciar el editor '{ $editor }'
error-editor-exited-unsuccessfully = El editor '{ $editor }' no terminó correctamente, por lo que el secreto no se modificó
error-editor-file-not-utf8 = El secreto editado no es UTF-8 válido
error-editor-file-create-failed = No se pudo crear un archivo temporal para el editor en '{ $path }'
editor-file-in-temp-dir = No están disponibles ni $XDG_RUNTIME_DIR ni /dev/shm, por lo que el secreto se escribe en un archivo en '{ $path }' mientras el editor está abierto. Ese directorio puede estar almacenado en disco.
editor-file-cleanup-failed = No se pudo sobrescribir y eliminar el archivo temporal del editor '{ $path }': { $error }
prompt-overwrite-secret = ¿Sobrescribir el secreto existente?
prompt-remove-secret = ¿Eliminar el secreto existente?
prompt-overwrite-one-time-password = ¿Sobrescribir la contraseña de un solo uso existente?
prompt-remove-one-time-password = ¿Eliminar la contraseña de un solo uso existente?

## Mensajes de error

error-cannot-get-user-confirmation = No se puede obtener la confirmación del usuario
error-no-confirmation-from-non-terminal = No se puede obtener la confirmación del usuario desde una entrada no terminal. Use --force para omitir la confirmación.
error-could-not-load-configuration = No se pudo cargar la configuración
error-store-does-not-exist = El almacén con nombre '{ $store_name }' no existe en la configuración
error-invalid-line-number = '{ $input }' no es un número de línea válido
error-line-number-must-not-be-zero = El número de línea no debe ser 0. Use 1 para la primera línea, -1 para la última
error-invalid-count = '{ $input }' no es un recuento válido
error-count-must-not-be-zero = El recuento no debe ser 0. Use 1 para omitir la primera línea
error-file-does-not-exist = El archivo '{ $input }' no existe
error-secret-already-exists = El secreto ya existe en { $secret_path }. Use --force para sobrescribirlo.
error-secret-already-exists-overwrite-or-inplace = El secreto ya existe en { $secret_path }. Use --force para sobrescribirlo completamente o --inplace para modificar solo su primera línea.
error-password-length-zero = La longitud de la contraseña no puede ser 0
error-password-length-too-short = La longitud de la contraseña es demasiado corta para los tipos de caracteres requeridos
error-password-no-character-kinds = Debe habilitarse al menos un tipo de carácter
error-password-generation-failed = No se puede generar una contraseña: { $reason }
error-no-secret-found = No se encontró ningún secreto en '{ $secret_path }'
error-not-allowed-to-remove-secret = No se permite eliminar el secreto en { $secret_path }. Use --force para sobrescribirlo.
error-secret-does-not-exist-for-edit = El secreto en { $secret_path } no existe. Use 'pasejo secret add' para crearlo.
error-one-time-password-already-exists = La contraseña de un solo uso ya existe en { $password_path }. Use --force para sobrescribirla.
error-not-allowed-to-remove-one-time-password = No se permite eliminar la contraseña de un solo uso en { $password_path }. Use --force para sobrescribirla.
error-no-one-time-password-found = No se encontró ninguna contraseña de un solo uso en '{ $password_path }'
error-invalid-one-time-password-secret = El secreto de la contraseña de un solo uso debe ser una cadena base32 no vacía (letras A-Z y dígitos 2-7)
error-invalid-otpauth-url = La URL otpauth no describe una contraseña de un solo uso válida
error-hotp-counter-exhausted = El contador HOTP ha alcanzado su valor máximo y no puede avanzar
error-no-store-in-configuration = No se encontró ningún almacén en la configuración. Ejecute 'pasejo store add ...' primero para añadir uno
error-unknown-store = El almacén '{ $store_name }' no existe en la configuración. Ejecute 'pasejo store list' para ver los almacenes configurados.
error-decrypt-requires-yes-i-know = Descifrado rechazado: esto imprimirá todos los secretos en texto plano en stdout. Pase --yes-i-know para confirmar.
error-no-store-or-global = No se encontró ningún almacén en la configuración y no se especificó la opción --global. Ejecute 'pasejo store add ...' primero para añadir uno
error-store-name-already-exists = El nombre del almacén ya existe. Por favor use un nombre diferente.
error-store-path-is-directory = No se puede usar un directorio como ruta del almacén. Por favor use una ruta de archivo.
error-cannot-create-store-path = No se puede crear la ruta del almacén. Por favor verifique la ruta y vuelva a intentarlo.
error-cannot-get-store-parent = No se puede obtener el directorio padre de la ruta del almacén. Por favor verifique la ruta y vuelva a intentarlo.
error-cannot-identify-store = No se puede identificar el almacén. Establezca un almacén predeterminado, use --store para especificar un almacén o use --global para establecer la identidad globalmente.
error-identity-file-already-added = valor no válido '{ $file }' para '--file <FILE>': el archivo ya se añadió como identidad
error-identity-file-not-known = valor no válido '{ $file }' para '--file <FILE>': el archivo no coincide con ninguna identidad conocida
error-identity-did-you-mean = ¿Quiso decir '{ $stored }'? Las rutas de identidad se comparan de forma exacta.
error-no-identity-files-to-decrypt = No hay archivos de identidad para descifrar. Añada al menos una identidad para completar la inicialización del almacén.
error-invalid-ssh-public-key-format = Formato de clave pública SSH no válido
error-username-empty = El nombre de usuario no debe estar vacío
error-username-contains-dotdot = Nombre de usuario no válido '{ $username }': no debe contener '..'
error-username-invalid-character = Nombre de usuario no válido '{ $username }': debe comenzar con una letra o dígito ASCII y contener solo letras ASCII, dígitos, '-', '_' y '.'
error-no-public-key-found-in-file = No se encontró ninguna clave pública en '{ $filename }'
error-no-public-keys-for-user = El usuario de { $provider } '{ $username }' no tiene claves SSH públicas para añadir como destinatarios
error-no-public-key-source = Debe especificar al menos una fuente para una clave pública
error-no-qrcode-found = No se encontró ningún código QR en '{ $qrcode }'
error-failed-to-decode-qrcode = Falló la decodificación del código QR en '{ $qrcode }'
error-qr-sandbox-failed = No se pudo ejecutar el decodificador de códigos QR en la zona de pruebas
error-qr-sandbox-child-signal = El decodificador de códigos QR fue terminado por la señal { $signal }
error-qr-sandbox-not-enforced = No se pudo aplicar la zona de pruebas del decodificador de códigos QR; el kernel en ejecución no admite Landlock
error-cannot-determine-store-name = No se puede determinar el nombre del almacén
error-cannot-parse-hook-command = No se puede analizar el comando: { $command }
error-empty-hook-command = Comando de hook vacío: { $command }
error-failed-to-run-hook = Falló la ejecución del hook { $command }
error-hook-failed-no-detail = el hook { $command } falló (salida { $exit })
error-hook-failed-with-detail = el hook { $command } falló (salida { $exit }): { $detail }
error-cannot-determine-store-parent-path = No se puede determinar el padre de la ruta del almacén { $path }
error-store-path-not-utf8 = No se puede sustituir %p en el token { $token }: la ruta del almacén { $path } no es UTF-8 válido
error-merge-conflict-recipients = Conflicto de fusión detectado en los destinatarios. Por favor, resuélvalo manualmente.
error-merge-conflict = Conflicto de fusión detectado en { $value_type }s. Por favor, resuélvalo manualmente.
error-recipient-not-found-in-store = Destinatario no encontrado en el almacén
error-cannot-remove-last-recipient = No se puede eliminar el último destinatario del almacén. Añada un nuevo destinatario antes de eliminar este.
error-cannot-decrypt-common-ancestor-store = No se puede descifrar el almacén ancestro común
error-cannot-decrypt-current-version-store = No se puede descifrar el almacén de la versión actual
error-cannot-decrypt-other-version-store = No se puede descifrar el almacén de la otra versión
error-cannot-read-file = No se puede leer el archivo en '{ $path }'
error-downloading-public-key-failed = Falló la descarga de la clave pública desde { $provider }
error-download-user-not-found = { $provider } no tiene ningún usuario '{ $username }'
error-download-rate-limited = { $provider } limitó la frecuencia de la solicitud; espera un momento e inténtalo de nuevo
error-download-server-error = { $provider } devolvió un error (HTTP { $status })
error-download-timeout = Se agotó el tiempo de espera al descargar la clave pública desde { $provider }; comprueba tu conexión o aumenta key-download-timeout-seconds
error-cannot-determine-parent-directory = No se puede determinar el directorio padre de { $path }
error-failed-to-create-directory = Falló la creación del directorio { $path }
error-failed-to-write-file = Falló la escritura de { $path }
error-failed-to-fsync-file = Falló fsync en { $path }
error-failed-to-rename-file = Falló el renombrado de { $from } a { $to }
error-failed-to-create-file = Falló la creación de { $path }
error-could-not-resolve-config-path = No se pudo resolver la ruta absoluta de la configuración
error-could-not-determine-config-path = No se pudo determinar la ruta de configuración
error-could-not-resolve-store-path = No se pudo resolver la ruta absoluta del almacén
error-could-not-create-config-dir = No se pudo crear el directorio de configuración
error-could-not-migrate-legacy-config = No se pudo migrar el archivo de configuración heredado
error-could-not-move-file = No se pudo mover el archivo
error-could-not-copy-file = No se pudo copiar el archivo
error-could-not-remove-source-after-copy = No se pudo eliminar el archivo fuente después de copiarlo
error-could-not-read-configuration = No se pudo leer la configuración
error-could-not-serialize-migrated-config = No se pudo serializar la configuración migrada
error-could-not-store-configuration = No se pudo almacenar la configuración
error-could-not-load-migrated-config = No se pudo cargar la configuración migrada
error-could-not-serialize-configuration = No se pudo serializar la configuración
error-config-not-valid-toml = El archivo de configuración en { $path } no es TOML válido
error-config-invalid-integer = Valor '{ $value }' no válido para la opción '{ $option }': se esperaba un entero no negativo
error-config-invalid-boolean = Valor '{ $value }' no válido para la opción '{ $option }': se esperaba uno de true, false, 1, 0, yes, no, y, n
error-could-not-open-store-for-lock = No se pudo abrir el archivo del almacén para bloquearlo: { $path }
error-could-not-acquire-store-lock = No se pudo adquirir el bloqueo del archivo del almacén: { $path }
error-store-lock-wait-timed-out = Se agotó el tiempo de espera del bloqueo del archivo del almacén: { $path }
error-cannot-encrypt-store = No se puede cifrar el almacén
error-failed-to-run-command = Falló la ejecución del comando { $binary }
error-command-exited-with = El comando { $binary } terminó con { $exit }
error-no-field-found = El secreto '{ $secret_path }' no tiene el campo '{ $field }'
error-no-line-found = El secreto '{ $secret_path }' no tiene la línea { $line }
error-value-unresolved = No se puede resolver el valor de '{ $name }'
error-env-assignment-invalid = '{ $input }' no tiene la forma VARIABLE=SECRETO
error-env-field-assignment-invalid = '{ $input }' no tiene la forma VARIABLE=SECRETO=CAMPO
error-env-variable-name-invalid = '{ $variable }' no es un nombre de variable de entorno válido. Use letras, dígitos y guiones bajos, y no empiece con un dígito
error-env-variable-duplicate = La variable de entorno { $variable } se asigna más de una vez
error-env-reference-line-and-field = La referencia para { $variable } define tanto 'line' como 'field'
error-resolve-input-invalid = No se pueden leer las referencias a secretos desde la entrada estándar
exec-value-too-short-to-redact = El valor de { $variable } tiene menos de { $length } caracteres y no se oculta
error-browser-host-requires-pipe = El host del navegador requiere una entrada estándar redirigida desde la extensión del navegador conectada. Ejecute este comando a través del manifiesto de mensajería nativa de su navegador, no directamente desde una terminal.
error-browser-store-file-missing = El archivo del almacén no existe: { $path }
error-browser-identity-passphrase-protected = El host del navegador no puede pedir la frase de contraseña de una identidad cifrada. Usa una identidad sin cifrar o respaldada por un plugin para este almacén.
prompt-browser-host-pinentry = PIN o frase de contraseña
error-browser-message-too-large = El mensaje de { $length } bytes supera el límite de mensajería nativa de { $limit } bytes
error-browser-response-too-large = La respuesta de { $length } bytes supera el límite de mensajería nativa de { $limit } bytes
error-browser-install-manifest-no-browser = Especifique al menos un --browser para el que instalar el manifiesto de mensajería nativa.
error-browser-install-manifest-needs-extension-id = El navegador '{ $browser }' requiere --extension-id porque los ID de extensión de los navegadores basados en Chromium se derivan de la clave pública de la extensión descomprimida.
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id no puede combinarse con más de un --browser, porque cada navegador necesita su propio id de extensión. Ejecute el comando una vez por navegador.
error-browser-install-manifest-unsupported-os = La instalación de manifiestos de mensajería nativa no está soportada en '{ $os }'.
error-browser-install-manifest-registry-failed = No se pudo escribir la clave de registro de native-messaging '{ $key }'
error-browser-install-manifest-no-home-dir = No se pudo determinar el directorio personal del usuario.
error-browser-install-manifest-no-data-dir = No se pudo determinar el directorio de datos del usuario.
error-browser-install-manifest-no-current-exe = No se pudo determinar la ruta del binario de pasejo en ejecución.
browser-install-manifest-written = Manifiesto de mensajería nativa para '{ $browser }' escrito en '{ $path }'
browser-install-manifest-print-wrapper-note = Nota: el manifiesto impreso hace referencia al script envoltorio en '{ $path }', que este comando no instaló. Use --binary para hacer referencia a un ejecutable existente en su lugar.
error-browser-install-manifest-invalid-env = La entrada --env '{ $entry }' no tiene el formato KEY=VALUE.
error-browser-install-manifest-invalid-env-name = El nombre --env '{ $name }' no es un nombre de variable de entorno POSIX válido (debe comenzar con una letra o guion bajo y contener solo letras, dígitos y guiones bajos).
error-browser-install-manifest-invalid-env-value-windows = El valor --env de '{ $name }' no se puede escribir en el wrapper de Windows: contiene una comilla doble o un salto de línea.
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' no es un nombre de host de mensajería nativa válido para { $browser }: debe estar formado por partes separadas por puntos de letras, dígitos y guiones bajos, en minúsculas para todos los navegadores excepto firefox.
error-browser-install-manifest-invalid-extension-id = --extension-id '{ $id }' no es un ID de extensión válido: solo puede contener letras ASCII, dígitos, '@', '.', '_', '-' y llaves.
error-browser-release-log-lock = No se pudo bloquear el registro de entregas al navegador: { $path }
error-browser-release-log-no-data-dir = No se pudo determinar el directorio de datos del registro de entregas al navegador.
error-browser-release-log-failed = No se pudo anotar la entrega en el registro de entregas al navegador, así que no se enviaron las credenciales.
error-browser-caller-not-allowed = La extensión del navegador '{ $caller }' no puede usar este host de pasejo. Ejecute `pasejo browser install-manifest` para esta extensión o añádala con --allowed-caller.
error-browser-caller-unnamed = El navegador no indicó qué extensión inició este host de pasejo, por lo que no se puede comprobar con --allowed-caller.
browser-log-empty = Todavía no se han entregado credenciales a ningún navegador.
browser-log-unreadable-lines = Líneas ilegibles omitidas del registro de entregas al navegador: { $count }
browser-log-header-time = Hora
browser-log-header-domain = Dominio
browser-log-header-store = Almacén
browser-log-header-secret = Secreto
browser-log-header-caller = Solicitante

## Remote stores
error-invalid-date = '{ $value }' no es una fecha con el formato AAAA-MM-DD.
error-remote-actions-failed = Algunas acciones pendientes fallaron.
error-remote-bad-request = El servidor no aceptó la solicitud.
error-remote-checkpoint-key-changed = La clave de checkpoint de { $url } difiere de la fijada en el primer inicio de sesión. Es posible que el servidor haya sido sustituido; consulte a sus operadores antes de volver a iniciar sesión.
error-remote-concurrent-change = Otra persona cambió '{ $name }' al mismo tiempo. Ejecute el comando de nuevo para aplicar su cambio a la nueva versión.
error-remote-conflict = El servidor tiene una versión más reciente. Ejecute el comando de nuevo.
error-remote-device-user-not-member = { $email } no es miembro de la organización. Añádalo primero con 'pasejo org role { $email } member'.
error-remote-fingerprint-mismatch = La huella no coincide. El servidor indica { $expected }.
error-remote-fingerprint-not-confirmed = La huella no se confirmó.
error-remote-fingerprint-required = Compare la huella { $expected } por otro canal y luego pásela con --fingerprint.
error-remote-forbidden = El servidor lo rechazó: no tiene permiso para esto.
error-remote-invalid-id = '{ $value }' no es un identificador válido.
error-remote-invalid-recipient = '{ $recipient }' no es un destinatario age al que el servidor pueda cifrar.
error-remote-login-denied = El inicio de sesión se denegó en el navegador.
error-remote-login-expired = El inicio de sesión no se aprobó a tiempo. Ejecute 'pasejo server login' de nuevo.
error-remote-needs-second-admin = Este cambio necesita la firma de un segundo administrador.
error-remote-no-data-dir = No existe un directorio de datos local para guardar las claves de este dispositivo. Defina PASEJO_DATA_DIR.
error-remote-no-org = No hay ninguna organización registrada. Ejecute 'pasejo org create' o 'pasejo org join'.
error-remote-no-recovery-recipient = La organización { $org } no tiene destinatario de recuperación.
error-remote-no-server = No hay ningún servidor registrado. Ejecute 'pasejo server login URL'.
error-remote-no-session = Este dispositivo no ha iniciado sesión.
error-remote-not-found = El servidor no conoce lo que se solicitó.
error-remote-not-logged-in = Este dispositivo no ha iniciado sesión en { $url }. Ejecute 'pasejo server login { $url }'.
error-remote-org-ambiguous = Varias organizaciones coinciden con '{ $org }'. Pase el identificador de la organización.
error-remote-org-required = Hay varias organizaciones registradas. Elija una con --org.
error-remote-pending-device = Este dispositivo espera la aprobación de un administrador o aprobador de la organización.
error-remote-rate-limited = El servidor pide esperar antes de volver a intentarlo.
error-remote-recipient-required = Ninguna identidad global proporciona un destinatario age para este dispositivo. Pase uno con --recipient.
error-remote-rejected = El registro de membresía de la organización rechazó el cambio.
error-remote-server-ambiguous = Hay varios servidores registrados. Indique la URL del servidor.
error-remote-server-failed = El servidor no pudo responder a la solicitud.
error-remote-session-rejected = El servidor ya no acepta la sesión de este dispositivo. Ejecute 'pasejo server login' de nuevo.
error-remote-signing-key-unreadable = No se puede leer la clave de firma del dispositivo en { $path }.
error-remote-store-command-not-applicable = El almacén '{ $store_name }' es remoto; los destinatarios, hooks y fusiones no se aplican a él.
error-remote-store-name-taken = Ya existe un almacén llamado '{ $name }'.
error-remote-store-required = El almacén '{ $store_name }' no es un almacén remoto.
error-remote-team-exists = Ya existe un equipo llamado '{ $name }'.
error-remote-too-large = La solicitud es más grande de lo que acepta el servidor.
error-remote-unavailable = A este dispositivo le falta lo que necesita la solicitud.
error-remote-unknown-device = No existe el dispositivo { $device }.
error-remote-unknown-org = Ninguna organización registrada coincide con '{ $org }'.
error-remote-unknown-proposal = Ninguna propuesta abierta tiene el identificador { $proposal }.
error-remote-unknown-server = No hay ningún servidor registrado en { $url }. Ejecute 'pasejo server login { $url }'.
error-remote-unknown-store = La organización no tiene el almacén { $store }.
error-remote-unknown-team = La organización no tiene ningún equipo llamado '{ $name }'.
error-remote-unknown-user = Nadie en la organización usa la dirección de correo { $email }.
error-remote-unreachable = No se puede contactar con el servidor.
error-remote-verification-failed = Lo que devolvió el servidor no se puede verificar. No confíe en este servidor hasta que sus operadores expliquen el motivo.
error-remote-workload-token-unreadable = No se puede leer el token de carga de trabajo en { $path }.
error-remote-wrong-object-kind = La entrada '{ $name }' contiene datos de un tipo inesperado.
prompt-remote-confirm-fingerprint = ¿Coincide la huella { $fingerprint } con la que se muestra en el otro lado?
remote-action-approve-device = aprobar el dispositivo '{ $label }' de { $email } (huella { $fingerprint })
remote-action-cosign-checkpoint = cofirmar el checkpoint de auditoría { $seq }
remote-action-join-member = añadir a { $email } como miembro
remote-action-remove-member = quitar a { $user } de la organización
remote-action-revoke-grant = revocar el permiso de { $principal } sobre el almacén { $store }
remote-action-rotate-epoch = rotar las claves del almacén { $store }
remote-action-sync-team = actualizar los miembros del equipo { $team }
remote-action-wrap-keys = dar a { $count } dispositivos las claves del almacén { $store }
remote-device-state-active = activo
remote-device-state-pending = pendiente
remote-action-done = hecho: { $action }
remote-action-skipped = omitido: { $action } ({ $reason })
remote-action-failed = falló: { $action }: { $error }
remote-action-pending = pendiente: { $action }
remote-audit-line = { $seq }  { $time }  { $actor }  { $action }  { $outcome }  { $store }  { $detail }
remote-device-line = { $id }  { $label }  { $email }  { $fingerprint }  { $state }
remote-proposal-line = { $proposal }  { $kind }  { $author }
remote-store-line = { $store }  { $label }  { $role }  { $local }
remote-rotation-item = { $name }
remote-status-server = { $url }: dispositivo { $device } ({ $label }), huella { $fingerprint }, { $session ->
    [yes] sesión iniciada
   *[other] sin sesión
}
remote-status-org = - organización { $name } ({ $org }), huella { $fingerprint }
remote-audit-verified = Se verificaron { $events } eventos de auditoría y { $checkpoints } checkpoints.
remote-change-proposed = El cambio { $kind } necesita un segundo administrador. La propuesta { $proposal } espera 'pasejo org cosign'.
remote-change-recorded = Se registró { $kind } en el registro de membresía.
remote-device-active = Este dispositivo está activo en { $org }.
remote-device-pending = Este dispositivo espera aprobación en { $org }. Dé a un administrador o aprobador su identificador { $device } y su huella { $fingerprint }.
remote-device-fingerprint = La huella de este dispositivo es { $fingerprint }.
remote-logged-in = Sesión iniciada en { $url } como { $email }.
remote-logged-out = Sesión cerrada en { $url }.
remote-login-visit = Abra { $url } en un navegador y confirme el código { $code }.
remote-no-actions = No hay acciones pendientes.
remote-no-proposals = No hay propuestas abiertas.
remote-no-servers = No hay ningún servidor registrado.
remote-no-stores = La organización no tiene ningún almacén visible para este dispositivo.
remote-org-created = Se creó la organización { $name } con el identificador { $org } y la huella { $fingerprint }.
remote-rotation-access-continues = { $user } todavía tiene acceso al almacén.
remote-rotation-nothing = { $user } no obtuvo nada que aún deba rotarse.
remote-settings-unchanged = La configuración ya tiene estos valores.
remote-automatic-approval-warning = Con la aprobación automática, `pasejo org actions --run` añade cada dispositivo que inscribe un miembro de la organización sin que nadie compare su huella. Un servidor o proveedor de identidad comprometido puede entonces inscribir un dispositivo propio y recibir las claves de los almacenes. Revisa `pasejo org devices` con regularidad.
remote-offline-reads-warning = Las lecturas sin conexión permiten a los dispositivos guardar una copia de los secretos y leerla sin el servidor. Esa lectura llega al registro de auditoría solo cuando el dispositivo la informa al volver a conectarse; un dispositivo perdido o comprometido no informa nada, y quitar a un miembro deja las copias en sus dispositivos. Permítelas solo donde leer sin red pese más que un registro de auditoría completo.
remote-audit-offline-reads-open = Las lecturas sin conexión están permitidas desde { $from }. Las lecturas de copias sin conexión aparecen en este registro solo en la medida en que los dispositivos las informaron.
remote-audit-offline-reads-closed = Las lecturas sin conexión estuvieron permitidas desde { $from } hasta { $until }. Las lecturas de copias sin conexión en ese período aparecen en este registro solo en la medida en que los dispositivos las informaron.
remote-device-approved-automatically = El dispositivo '{ $label }' se aprobó sin que nadie comparara su huella, porque la organización aprueba los dispositivos automáticamente.
remote-store-added = Se registró el almacén remoto { $label } como '{ $name }'.
remote-store-created = Se creó el almacén remoto '{ $name }' con el identificador { $store }.
remote-store-migrated = Se copiaron { $count } entradas de '{ $name }' al servidor. El almacén ahora apunta a la copia remota; el archivo { $path } no ha cambiado.
remote-store-recovered = Se recuperaron { $count } entradas de { $label } en { $path }.
remote-store-rekeyed = Se reescribieron { $count } entradas con las claves actuales.

## Remote administration
error-remote-emergency-unregistered = El inicio de sesión de emergencia necesita un dispositivo ya registrado en { $url }.
error-remote-unknown-review = Ninguna revisión de acceso tiene el identificador { $review }.
error-remote-not-under-review = { $principal } no tiene ningún permiso en la revisión { $review }.
remote-emergency-logged-in = Sesión iniciada en { $url } como { $email } mediante la ventana de emergencia, hasta { $expires }.
remote-scim-token-created = Token SCIM { $id } creado. Se muestra abajo una sola vez y no puede volver a mostrarse.
remote-scim-token-line = { $id }  { $created }
remote-no-scim-tokens = No hay tokens SCIM vigentes.
remote-scim-token-revoked = Token SCIM { $id } revocado.
remote-review-opened = Revisión de acceso { $review } abierta para el almacén { $store }, vence el { $due }.
remote-review-line = { $review }  { $store }  { $due }  { $state }
remote-review-item = - { $principal }  { $role }  { $decision }
remote-review-state-open = abierta
remote-review-state-complete = completada
remote-review-decision-keep = mantener
remote-review-decision-revoke = revocar
remote-review-decision-pending = pendiente
remote-no-reviews = No hay revisiones de acceso.
remote-review-decided = Se registró '{ $decision }' para { $principal } en la revisión { $review }.
remote-review-revokes-pending = Las decisiones de revocación surten efecto cuando un administrador de la organización o del almacén ejecuta 'pasejo org actions --run'.
