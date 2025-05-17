```
$ PASEJO_CONFIG=config.toml pasejo store add --path store --name something
info: Store 'something' added at '[CWD]/store'

$ PASEJO_CONFIG=config.toml pasejo recipient add --public-key dgdsfgdfg
? 1
Error: Cannot encrypt store

Caused by:
    Invalid recipient '⁨dgdsfgdfg⁩'.

```
