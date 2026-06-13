Username validation happens before any network request, so this case is
fully offline. The successful download path is covered hermetically in
`tests/recipient_download_tests.rs`, which serves a fixture `.keys` body
over a local HTTP server via the `GITLAB_HOST` override — keeping the
default suite off the live internet and off any maintainer's account.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo recipient add --gitlab sebastian/hoss
? 1
Error: Invalid username 'sebastian/hoss': must start with an ASCII letter or digit and contain only ASCII letters, digits, '-', '_' and '.'

```
