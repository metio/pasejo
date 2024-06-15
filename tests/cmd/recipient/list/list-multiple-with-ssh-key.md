```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo identity add --file some-identity
info: Identity using file '[CWD]/some-identity' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
info: Recipient for 'age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB Some Name here' --name 'Overwritten Name'
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB' added

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr Sebastian Hoß'
info: Recipient for 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr' added

$ PASEJO_CONFIG=config.toml pasejo recipient list
age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
# Overwritten Name
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB
# Sebastian Hoß
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA/FenZ/57gW+roJr8DaWAE94QJapctVF4eBugQDOAFr

```
