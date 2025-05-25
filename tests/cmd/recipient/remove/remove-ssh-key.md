```
$ pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --gitlab sebastian.hoss
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' added

$ pasejo recipient remove 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr'
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' removed

```
