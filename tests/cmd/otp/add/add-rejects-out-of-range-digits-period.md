`otp add` range-checks `--digits` (6..=8) and `--period` (>=1) at parse
time — before any store is touched — so a bad value fails here, where the
user can fix it, instead of writing an entry that dead-ends every later
`otp show`.

```
$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU --digits 4 too-few-digits
? 2
error: invalid value '4' for '--digits <DIGITS>': 4 is not in 6..=8

For more information, try '--help'.

$ pasejo otp add --type totp --secret JEQDYMZAN5YGK3RAONXXK4TDMU --period 0 zero-period
? 2
error: invalid value '0' for '--period <PERIOD>': 0 is not in 1..18446744073709551615

For more information, try '--help'.

```
