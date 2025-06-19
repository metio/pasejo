# Add one-time passwords (OTP)

One-time passwords (OTP) are part of a store and shared with all users of the store. They can be used to generate time-based or counter-based one-time passwords. You can add an OTP to a specific store using the [pasejo otp add](../commands/pasejo-cmd-otp.md) command.

## Add a TOTP manually

You can add a TOTP to your store like this:

```shell
$ pasejo otp add --type totp --secret TOTP_SECRET_CODE some-name
```

This will add a TOTP with the secret code `TOTP_SECRET_CODE` and the name `some-name` to your store.

## Add a TOTP from a URL

You can also add a TOTP from a URL like this:

```shell
$ pasejo otp add --url otpauth://totp/Example:alice@example.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&issuer=Example some-name
```

## Add a TOTP from a QR code

You can also add a TOTP from a QR code. To do this, you need to save the QR code as an image file and then run the following command:

```shell
$ pasejo otp add --qrcode qrcode.png some-name
```

## Add a HOTP manually

You can add a HOTP to your store like this:

```shell
$ pasejo otp add --type hotp --secret HOTP_SECRET --counter 1 some-name
```

This will add a HOTP with the secret code `HOTP_SECRET`, a counter value of `1`, and the name `some-name` to your store.

## Add a HOTP from a URL

You can also add a HOTP from a URL like this:

```shell
$ pasejo otp add --url otpauth://hotp/Example:alice@example.com?secret=JEQDYMZAN5YGK3RAONXXK4TDMU&counter=1&issuer=Example some-name
```

## Add a HOTP from a QR code

You can also add a HOTP from a QR code. To do this, you need to save the QR code as an image file and then run the following command:

```shell
$ pasejo otp add --qrcode qrcode.png some-name
```
