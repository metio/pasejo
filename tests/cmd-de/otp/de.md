```
$ pasejo store add --path store --name something
[..]

$ pasejo identity add --file some-identity --store something
[..]

$ pasejo recipient add --public-key age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd
[..]

$ pasejo otp add --type hotp --secret JEQDYMZAN5YGK3RAONXXK4TDMU --counter 18446744073709551615 exhausted
[..]

$ pasejo otp show exhausted
? 1
Error: Der HOTP-Zähler hat seinen Höchstwert erreicht und kann nicht weiter erhöht werden

```
