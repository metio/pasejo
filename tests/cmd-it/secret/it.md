```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo secret generate --length 0 zero
? 1
Error: La lunghezza della password non può essere 0

$ pasejo secret generate --length 2 short
? 1
Error: La lunghezza della password è troppo corta per i tipi di carattere richiesti

$ pasejo secret generate --no-numbers --no-lowercase-letters --no-uppercase-letters none
? 1
Error: Deve essere abilitato almeno un tipo di carattere

$ pasejo secret generate edited
[..]

$ pasejo secret edit edited
? 1
...
Error: Impossibile leggere il segreto per edited

Caused by:
    L'editor 'false' non è terminato correttamente, quindi il segreto non è stato modificato

```
