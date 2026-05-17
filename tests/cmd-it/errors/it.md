```
$ pasejo secret add some-secret
? 1
Error: Nessun archivio trovato nella configurazione. Eseguire prima 'pasejo store add ...' per aggiungerne uno

$ pasejo store add --path store --name primary
Archivio 'primary' aggiunto in '[CWD]/store'

$ pasejo store add --path other --name primary
? 1
Error: Il nome dell'archivio esiste già. Usare un nome diverso.

$ pasejo store set-default missing
? 2
error: invalid value 'missing' for '<NAME>': L'archivio con nome 'missing' non esiste nella configurazione

For more information, try '--help'.

$ pasejo config set ignore-missing-identities tru
? 1
Error: Invalid boolean value 'tru'. Expected one of: true, false, 1, 0, yes, no, y, n

```
