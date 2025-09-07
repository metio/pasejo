```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo hook set --push 'something'

$ pasejo hook get
store push: something

$ pasejo hook set --push 'first' --push 'second'

$ pasejo hook get
store push: first
store push: second

$ pasejo hook set --pull 'something'

$ pasejo hook get
store pull: something

$ pasejo hook set --pull 'pull' --push 'push'

$ pasejo hook get
store pull: pull
store push: push

$ pasejo hook set --pull 'append pull' --push 'append push' --append

$ pasejo hook get
store pull: pull
store pull: append pull
store push: push
store push: append push

$ pasejo hook set --pull 'prepend pull' --push 'prepend push' --prepend

$ pasejo hook get
store pull: prepend pull
store pull: pull
store pull: append pull
store push: prepend push
store push: push
store push: append push

$ pasejo hook set --pull 'global pull' --push 'global push' --global

$ pasejo hook get
global pull: global pull
store pull: prepend pull
store pull: pull
store pull: append pull
global push: global push
store push: prepend push
store push: push
store push: append push

$ pasejo hook set --global

$ pasejo hook get
store pull: prepend pull
store pull: pull
store pull: append pull
store push: prepend push
store push: push
store push: append push

$ pasejo hook set

$ pasejo hook get

$ pasejo hook set --pull 'some' --push 'value' --prepend --append
? 2
error: the argument '--prepend' cannot be used with '--append'

Usage: pasejo hook set --pull <PULL> --push <PUSH> --prepend

For more information, try '--help'.

$ pasejo hook set --pull 'some' --push 'value' --append --prepend
? 2
error: the argument '--append' cannot be used with '--prepend'

Usage: pasejo hook set --pull <PULL> --push <PUSH> --append

For more information, try '--help'.

$ pasejo hook set --pull 'global pull' --push 'global push' --global --store something
? 2
error: the argument '--global' cannot be used with '--store <STORE>'

Usage: pasejo hook set --pull <PULL> --push <PUSH> --global

For more information, try '--help'.

$ pasejo hook set --pull 'global pull' --push 'global push' --store something --global
? 2
error: the argument '--store <STORE>' cannot be used with '--global'

Usage: pasejo hook set --pull <PULL> --push <PUSH> --store <STORE>

For more information, try '--help'.

```
