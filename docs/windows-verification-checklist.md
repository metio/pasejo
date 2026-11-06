# Windows verification checklist

The browser integration's Windows path (`pasejo browser install-manifest`'s
registry + `.cmd` registration, and the extension talking to
`pasejo browser host` through it) is compiled and unit-tested on every pull
request by the `x86_64-pc-windows-msvc` job on a `windows-latest` runner, and
the same job runtime-checks the host registration (see below). The parts that need
a real browser are listed here for a human to run on a Windows machine.

## Automated (CI + local)

The **Verify Windows native-messaging registration** step in
`.github/workflows/verify.yml` runs `dev/verify-windows-install-manifest.ps1`
on `windows-latest`. It runs `install-manifest` and asserts the three
artifacts the registration must produce:

- the `HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts\<host>`
  value pointing at a manifest file,
- the manifest JSON (correct `name`, `type=stdio`, `allowed_origins`, and a
  `.cmd` `path`), and
- the `.cmd` wrapper that launches `pasejo browser host` with the extension's
  origin as `--allowed-caller`.

`cargo test` also runs natively on the Windows runner, where the
`cfg!(windows)` branches of `wrapper_path`, `wrapper_content` and
`manifest_path_for_browser` take the Windows side.

To run the registration check on your own Windows checkout:

```powershell
pwsh ./dev/verify-windows-install-manifest.ps1
```

## Manual (real browser — not automatable in CI)

Prerequisites: a Windows build of `pasejo` on `PATH`, an `age` key loaded into
the OpenSSH agent (so the host can decrypt without a passphrase prompt), and a
store with at least one identity, recipient, and a secret that carries a `url`
field for a site you can log into.

- [ ] **Register the host** for your browser, e.g.
      `pasejo browser install-manifest --browser edge --extension-id <id>`
      (the id is shown on `edge://extensions` after the unpacked
      `dist/chromium` extension loads). Repeat for `chrome` / `firefox` as
      needed.
- [ ] **`pasejo browser host` launches and speaks the protocol.** With the
      extension installed, opening the popup must reach the host — confirm the
      popup lists matches rather than a connection error. (This is the real
      test of the `.cmd` wrapper ↔ binary-stdio handshake, and of the
      `chrome-extension://<id>/` and `--parent-window=<handle>` arguments
      reaching the host through `%*`, which CI cannot cover.)
- [ ] **Popup fill** injects username + password into a login form.
- [ ] **Per-host store memory**: pick a non-default store for a site, fill,
      reopen the popup on that site — the store is restored.
- [ ] **Multi-store streaming**: with more than one store configured, the *All
      stores* entry fans out and renders results as each store answers.
- [ ] **Basic-auth opt-in (default off)**: enable the popup's basic-auth
      toggle, visit a site that returns `401` with a single matching stored
      credential — it is supplied automatically; visit one with zero or
      multiple matches — the browser's native dialog appears instead.
- [ ] **Uninstall is clean**: re-running `install-manifest` re-points the
      registry value; removing the registry key + manifest + `.cmd` stops the
      browser from launching the host.

When all manual boxes pass on a real Windows machine, drop the note in
`CLAUDE.md` that no real browser has been driven through the Windows host,
and the "not yet exercised on a real Windows machine" caveat in the
web-extension README.
