```
$ pasejo browser install-manifest --browser firefox --print --binary /usr/local/bin/pasejo --host-name ../escape
? 1
Error: --host-name '../escape' is not a valid native-messaging host name for firefox: it must be dot-separated parts of letters, digits, and underscores, in lowercase for every browser except firefox.

$ pasejo browser install-manifest --browser chrome --extension-id abcdefghijklmnop --print --binary /usr/local/bin/pasejo --host-name Wtf.Metio
? 1
Error: --host-name 'Wtf.Metio' is not a valid native-messaging host name for chrome: it must be dot-separated parts of letters, digits, and underscores, in lowercase for every browser except firefox.

```
