```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: La longitud de la contraseña no puede ser 0

$ pasejo secret generate --length 2 short
? 1
Error: La longitud de la contraseña es demasiado corta para los tipos de caracteres requeridos

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: Debe habilitarse al menos un tipo de carácter

$ pasejo secret generate edited
[..]

$ pasejo secret edit edited
? 1
...
Error: No se pudo leer el secreto para edited

Caused by:
    El editor 'false' no terminó correctamente, por lo que el secreto no se modificó

```
