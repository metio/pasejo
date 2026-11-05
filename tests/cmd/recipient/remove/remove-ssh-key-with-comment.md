```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr alice@laptop'
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' added

$ pasejo recipient remove 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr alice@laptop'
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' removed

$ pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr '
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' added

$ pasejo recipient remove 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr'
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' removed

$ pasejo recipient remove ' age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd '
? 1
Error: Cannot remove the last recipient from the store. Please add a new recipient before removing this one.

```
