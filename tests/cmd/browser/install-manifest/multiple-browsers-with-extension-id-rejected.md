A single `--extension-id` cannot be disambiguated across multiple
browsers — a chromium id written into Firefox's gecko-id slot (or vice
versa) yields a manifest that never authorizes the real extension. The
command rejects the combination up front; run it once per browser.

```
$ pasejo browser install-manifest --browser firefox --browser chrome --extension-id abcdefghijklmnop
? 1
Error: --extension-id cannot be combined with more than one --browser, because each browser needs its own extension id. Run the command once per browser.

```
