`otp add` without `--url` or `--qrcode` requires a `--secret` that decodes
as non-empty base32, so a missing or mistyped secret fails here instead of
producing wrong codes or dead-ending every later `otp show`.

```
$ pasejo otp add --type totp missing-secret
? 2
error: the following required arguments were not provided:
  --secret <SECRET>

Usage: pasejo otp add --type <OTP_TYPE> --secret <SECRET> <PASSWORD_PATH>

For more information, try '--help'.

$ pasejo otp add --type totp --secret "" empty-secret
? 1
Error: The one-time password secret must be a non-empty base32 string (letters A-Z and digits 2-7)

$ pasejo otp add --type totp --secret JBSWY3DPEHPK3PX0 not-base32
? 1
Error: The one-time password secret must be a non-empty base32 string (letters A-Z and digits 2-7)

```
