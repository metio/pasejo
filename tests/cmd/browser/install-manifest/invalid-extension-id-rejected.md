Extension ids end up quoted in the generated wrapper, so the command
accepts only the characters chromium ids and Firefox add-on ids consist
of.

```
$ pasejo browser install-manifest --browser firefox --extension-id 'evil" & calc' --print --binary /usr/local/bin/pasejo
? 1
Error: --extension-id 'evil" & calc' is not a valid extension id: it may contain only ASCII letters, digits, '@', '.', '_', '-' and curly braces.

```
