```
$ pasejo otp help add
Adds a one-time password

Usage: pasejo otp add [OPTIONS] <PASSWORD_PATH>

Arguments:
  <PASSWORD_PATH>  The path of the one-time password within the selected store

Options:
  -s, --store <STORE>          Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                  Overwrite an existing one-time password without prompting
      --url <URL>              Parse an otpauth URL
      --qrcode <QRCODE>        Parse a QR code containing an otpauth URL
      --secret <SECRET>        The base secret of the one-time password
      --type <OTP_TYPE>        The type of the one-time password [possible values: totp, hotp]
  -v, --verbose...             Increase logging verbosity
      --algorithm <ALGORITHM>  The algorithm of the one-time password [possible values: sha1, sha256, sha512]
  -q, --quiet...               Decrease logging verbosity
      --digits <DIGITS>        The digits of the one-time password
      --period <PERIOD>        The period of the one-time password
      --skew <SKEW>            The skew of the one-time password
      --counter <COUNTER>      The counter of the one-time password
  -h, --help                   Print help

```

```
$ pasejo otp add --help
Adds a one-time password

Usage: pasejo otp add [OPTIONS] <PASSWORD_PATH>

Arguments:
  <PASSWORD_PATH>  The path of the one-time password within the selected store

Options:
  -s, --store <STORE>          Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                  Overwrite an existing one-time password without prompting
      --url <URL>              Parse an otpauth URL
      --qrcode <QRCODE>        Parse a QR code containing an otpauth URL
      --secret <SECRET>        The base secret of the one-time password
      --type <OTP_TYPE>        The type of the one-time password [possible values: totp, hotp]
  -v, --verbose...             Increase logging verbosity
      --algorithm <ALGORITHM>  The algorithm of the one-time password [possible values: sha1, sha256, sha512]
  -q, --quiet...               Decrease logging verbosity
      --digits <DIGITS>        The digits of the one-time password
      --period <PERIOD>        The period of the one-time password
      --skew <SKEW>            The skew of the one-time password
      --counter <COUNTER>      The counter of the one-time password
  -h, --help                   Print help

```

```
$ pasejo otp add -h
Adds a one-time password

Usage: pasejo otp add [OPTIONS] <PASSWORD_PATH>

Arguments:
  <PASSWORD_PATH>  The path of the one-time password within the selected store

Options:
  -s, --store <STORE>          Optional name of store to use. Defaults to the default store or the first one defined in the local user configuration
  -f, --force                  Overwrite an existing one-time password without prompting
      --url <URL>              Parse an otpauth URL
      --qrcode <QRCODE>        Parse a QR code containing an otpauth URL
      --secret <SECRET>        The base secret of the one-time password
      --type <OTP_TYPE>        The type of the one-time password [possible values: totp, hotp]
  -v, --verbose...             Increase logging verbosity
      --algorithm <ALGORITHM>  The algorithm of the one-time password [possible values: sha1, sha256, sha512]
  -q, --quiet...               Decrease logging verbosity
      --digits <DIGITS>        The digits of the one-time password
      --period <PERIOD>        The period of the one-time password
      --skew <SKEW>            The skew of the one-time password
      --counter <COUNTER>      The counter of the one-time password
  -h, --help                   Print help

```
