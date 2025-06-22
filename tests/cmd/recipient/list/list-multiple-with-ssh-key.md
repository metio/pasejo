```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity
Identity using file '[CWD]/some-identity' added

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB Some Name here' --name 'Overwritten Name'
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB' added

$ pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr Sebastian Hoß'
Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' added

$ pasejo recipient list
age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
# Overwritten Name
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB
# Sebastian Hoß
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr

```
