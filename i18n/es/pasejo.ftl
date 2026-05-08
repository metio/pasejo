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
