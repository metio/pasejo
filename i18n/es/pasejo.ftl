# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## Destinatarios

recipient-added = Destinatario para '{ $public_key }' añadido
recipient-removed = Destinatario para '{ $public_key }' eliminado
recipient-does-not-exist-ignored = El destinatario para '{ $public_key }' no existe en el almacén – se omite

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

## Ejecución de hooks

execute-pull-hooks = Ejecutando hooks de pull para el almacén '{ $store_name }'
execute-push-hooks = Ejecutando hooks de push para el almacén '{ $store_name }'

## Conflictos de fusión

merge-conflict-recipient-names = Conflicto de fusión para destinatario con clave pública '{ $public_key }': los nombres '{ $first_name }' y '{ $second_name }' difieren
merge-conflict-recipient-removed-and-renamed = Conflicto de fusión para destinatario con clave pública '{ $public_key }': el destinatario fue eliminado en una versión y renombrado a '{ $new_name }' en la otra
merge-conflict-values = Conflicto de fusión para { $value_type } en '{ $secret_path }': los valores difieren entre las dos versiones
merge-conflict-removed-and-modified = Conflicto de fusión para { $value_type } en '{ $secret_path }': { $value_type } fue eliminado en una versión y modificado en la otra

## Salida de listas (stdout)

list-global-identity = global: { $identity_file }
list-store-identity = almacén: { $identity_file }
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
error-no-secret-found = No se encontró ningún secreto en '{ $secret_path }'
error-not-allowed-to-remove-secret = No se permite eliminar el secreto en { $secret_path }. Use --force para sobrescribirlo.
error-secret-does-not-exist-for-edit = El secreto en { $secret_path } no existe. Use 'pasejo secret add' para crearlo.
error-one-time-password-already-exists = La contraseña de un solo uso ya existe en { $password_path }. Use --force para sobrescribirla.
error-not-allowed-to-remove-one-time-password = No se permite eliminar la contraseña de un solo uso en { $password_path }. Use --force para sobrescribirla.
error-no-one-time-password-found = No se encontró ninguna contraseña de un solo uso en '{ $password_path }'
error-no-store-in-configuration = No se encontró ningún almacén en la configuración. Ejecute 'pasejo store add ...' primero para añadir uno
error-decrypt-requires-yes-i-know = Descifrado rechazado: esto imprimirá todos los secretos en texto plano en stdout. Pase --yes-i-know para confirmar.
error-no-store-or-global = No se encontró ningún almacén en la configuración y no se especificó la opción --global. Ejecute 'pasejo store add ...' primero para añadir uno
error-store-name-already-exists = El nombre del almacén ya existe. Por favor use un nombre diferente.
error-store-path-is-directory = No se puede usar un directorio como ruta del almacén. Por favor use una ruta de archivo.
error-cannot-create-store-path = No se puede crear la ruta del almacén. Por favor verifique la ruta y vuelva a intentarlo.
error-cannot-get-store-parent = No se puede obtener el directorio padre de la ruta del almacén. Por favor verifique la ruta y vuelva a intentarlo.
error-cannot-identify-store = No se puede identificar el almacén. Establezca un almacén predeterminado, use --store para especificar un almacén o use --global para establecer la identidad globalmente.
error-no-identity-files-to-decrypt = No hay archivos de identidad para descifrar. Añada al menos una identidad para completar la inicialización del almacén.
error-invalid-ssh-public-key-format = Formato de clave pública SSH no válido
error-username-empty = El nombre de usuario no debe estar vacío
error-username-contains-dotdot = Nombre de usuario no válido '{ $username }': no debe contener '..'
error-username-invalid-character = Nombre de usuario no válido '{ $username }': debe comenzar con una letra o dígito ASCII y contener solo letras ASCII, dígitos, '-', '_' y '.'
error-no-public-key-found-in-file = No se encontró ninguna clave pública en '{ $filename }'
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
error-recipient-not-found-in-store = Destinatario no encontrado en el almacén
error-cannot-decrypt-common-ancestor-store = No se puede descifrar el almacén ancestro común
error-cannot-decrypt-current-version-store = No se puede descifrar el almacén de la versión actual
error-cannot-decrypt-other-version-store = No se puede descifrar el almacén de la otra versión
error-cannot-read-file = No se puede leer el archivo en '{ $path }'
error-downloading-public-key-failed = Falló la descarga de la clave pública desde { $provider }
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
error-could-not-open-store-for-lock = No se pudo abrir el archivo del almacén para bloquearlo: { $path }
error-could-not-acquire-store-lock = No se pudo adquirir el bloqueo del archivo del almacén: { $path }
error-cannot-encrypt-store = No se puede cifrar el almacén
error-failed-to-run-command = Falló la ejecución del comando { $binary }
error-command-exited-with = El comando { $binary } terminó con { $exit }
