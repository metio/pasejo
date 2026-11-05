```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: La longueur du mot de passe ne doit pas être 0

$ pasejo secret generate --length 2 short
? 1
Error: La longueur du mot de passe est trop courte pour les types de caractères requis

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: Au moins un type de caractère doit être activé

```
