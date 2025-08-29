# Using pasejo with systemd

In case you want to automatically update your store(s) whenever your computer boots, try the following systemd service which activates after you have logged in to your computer:

```unit file (systemd)
[Unit]
Description=Update pasejo stores
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/bin/pasejo store sync --all --pull
RemainAfterExit=false

[Install]
WantedBy=default.target
```

Adjust the path to `pasejo` to the one used on your system! See `pasejo store sync --help` for all available options.
