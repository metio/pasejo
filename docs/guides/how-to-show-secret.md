# Showing secrets

To show a secret, you can use the `pasejo secret show` command followed by the name of the secret. For example, to show a secret named `my-secret`, you would run:

```shell
$ pasejo secret show my-secret
```

This command will display the value of the secret in the terminal.

## QR Code Display

If you want to display the secret as a QR code, you can use the `--qrcode` option:

```shell
$ pasejo secret show my-secret --qrcode
```

This will generate a QR code representation of the secret, which can be scanned with a QR code reader.

## Copy to Clipboard

To copy the secret to your clipboard, you can use the `--clip` option:

```shell
$ pasejo secret show my-secret --clip
```

This will copy the secret value to your clipboard, allowing you to paste it elsewhere.

## Show Specific Line

If the secret contains multiple lines, and you want to show only a specific line, you can use the `--line` option followed by the line number. For example, to show the second line of the secret:

```shell
$ pasejo secret show my-secret --line 2
```

If you want to skip lines, you can use a negative number. For example, to skip the first line and show the rest:

```shell
$ pasejo secret show my-secret --line=-1
```
