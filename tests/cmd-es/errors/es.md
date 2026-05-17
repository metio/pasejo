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
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
