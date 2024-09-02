```console
$ pasejo completion zsh
#compdef pasejo

autoload -U is-at-least

_pasejo() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : /
'-s+[Optional name of store to use. Defaults to the first store defined in the local user configuration]:STORE: ' /
'--store=[Optional name of store to use. Defaults to the first store defined in the local user configuration]:STORE: ' /
'-h[Print help]' /
'--help[Print help]' /
":: :_pasejo_commands" /
"*::: :->pasejo" /
&& ret=0
    case $state in
    (pasejo)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-command-$line[1]:"
        case $line[1] in
            (completion)
_arguments "${_arguments_options[@]}" : /
'-h[Print help]' /
'--help[Print help]' /
':shell:(bash elvish fish powershell zsh)' /
&& ret=0
;;
(identity)
_arguments "${_arguments_options[@]}" : /
'-h[Print help]' /
'--help[Print help]' /
":: :_pasejo__identity_commands" /
"*::: :->identity" /
&& ret=0

    case $state in
    (identity)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-identity-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
'-f+[The path to the identity file]:FILE:_files' /
'--file=[The path to the identity file]:FILE:_files' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
'-f+[The path to the identity file]:FILE:_files' /
'--file=[The path to the identity file]:FILE:_files' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__identity__help_commands" /
"*::: :->help" /
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-identity-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(recipient)
_arguments "${_arguments_options[@]}" : /
'-h[Print help]' /
'--help[Print help]' /
":: :_pasejo__recipient_commands" /
"*::: :->recipient" /
&& ret=0

    case $state in
    (recipient)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-recipient-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
'-k+[The public key of the new recipient]:PUBLIC_KEY: ' /
'--public-key=[The public key of the new recipient]:PUBLIC_KEY: ' /
'-n+[The name of the new recipient]:NAME: ' /
'--name=[The name of the new recipient]:NAME: ' /
'-p+[The path to a folder or secret that should be readable by the given recipient]:PATH:_files' /
'--path=[The path to a folder or secret that should be readable by the given recipient]:PATH:_files' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
'-k+[The public key of the recipient to remove]:PUBLIC_KEY: ' /
'--public-key=[The public key of the recipient to remove]:PUBLIC_KEY: ' /
'-p+[The path to a folder or secret that should no longer be readable by the given recipient]:PATH:_files' /
'--path=[The path to a folder or secret that should no longer be readable by the given recipient]:PATH:_files' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(inherit)
_arguments "${_arguments_options[@]}" : /
'-p+[The path to a folder or secret that should inherit its recipients from its parent]:PATH:_files' /
'--path=[The path to a folder or secret that should inherit its recipients from its parent]:PATH:_files' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__recipient__help_commands" /
"*::: :->help" /
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-recipient-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(inherit)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(store)
_arguments "${_arguments_options[@]}" : /
'-h[Print help]' /
'--help[Print help]' /
":: :_pasejo__store_commands" /
"*::: :->store" /
&& ret=0

    case $state in
    (store)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-store-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : /
'-p+[The path on your local system for the new store]:PATH:_files -/' /
'--path=[The path on your local system for the new store]:PATH:_files -/' /
'-a+[The alias for the new store]:ALIAS: ' /
'--alias=[The alias for the new store]:ALIAS: ' /
'-v+[The version control system to use]:VCS:(none git mercurial)' /
'--vcs=[The version control system to use]:VCS:(none git mercurial)' /
'-h[Print help]' /
'--help[Print help]' /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__store__help_commands" /
"*::: :->help" /
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-store-help-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(help)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__help_commands" /
"*::: :->help" /
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-help-command-$line[1]:"
        case $line[1] in
            (completion)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(identity)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__help__identity_commands" /
"*::: :->identity" /
&& ret=0

    case $state in
    (identity)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-help-identity-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
(recipient)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__help__recipient_commands" /
"*::: :->recipient" /
&& ret=0

    case $state in
    (recipient)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-help-recipient-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
(inherit)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
(store)
_arguments "${_arguments_options[@]}" : /
":: :_pasejo__help__store_commands" /
"*::: :->store" /
&& ret=0

    case $state in
    (store)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:pasejo-help-store-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
(help)
_arguments "${_arguments_options[@]}" : /
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_pasejo_commands] )) ||
_pasejo_commands() {
    local commands; commands=(
'completion:Generate shell completions' /
'identity:Manage identities' /
'recipient:Manage recipients' /
'store:Manage stores' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo commands' commands "$@"
}
(( $+functions[_pasejo__completion_commands] )) ||
_pasejo__completion_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo completion commands' commands "$@"
}
(( $+functions[_pasejo__help_commands] )) ||
_pasejo__help_commands() {
    local commands; commands=(
'completion:Generate shell completions' /
'identity:Manage identities' /
'recipient:Manage recipients' /
'store:Manage stores' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo help commands' commands "$@"
}
(( $+functions[_pasejo__help__completion_commands] )) ||
_pasejo__help__completion_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help completion commands' commands "$@"
}
(( $+functions[_pasejo__help__help_commands] )) ||
_pasejo__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help help commands' commands "$@"
}
(( $+functions[_pasejo__help__identity_commands] )) ||
_pasejo__help__identity_commands() {
    local commands; commands=(
'add:Adds an identity' /
'remove:Remove an identity' /
    )
    _describe -t commands 'pasejo help identity commands' commands "$@"
}
(( $+functions[_pasejo__help__identity__add_commands] )) ||
_pasejo__help__identity__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help identity add commands' commands "$@"
}
(( $+functions[_pasejo__help__identity__remove_commands] )) ||
_pasejo__help__identity__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help identity remove commands' commands "$@"
}
(( $+functions[_pasejo__help__recipient_commands] )) ||
_pasejo__help__recipient_commands() {
    local commands; commands=(
'add:Adds a recipient' /
'remove:Remove a recipient' /
'inherit:Removes the recipients of a folder or secret so that it inherits its recipients from its parent' /
    )
    _describe -t commands 'pasejo help recipient commands' commands "$@"
}
(( $+functions[_pasejo__help__recipient__add_commands] )) ||
_pasejo__help__recipient__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help recipient add commands' commands "$@"
}
(( $+functions[_pasejo__help__recipient__inherit_commands] )) ||
_pasejo__help__recipient__inherit_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help recipient inherit commands' commands "$@"
}
(( $+functions[_pasejo__help__recipient__remove_commands] )) ||
_pasejo__help__recipient__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help recipient remove commands' commands "$@"
}
(( $+functions[_pasejo__help__store_commands] )) ||
_pasejo__help__store_commands() {
    local commands; commands=(
'init:Initialize a new store' /
    )
    _describe -t commands 'pasejo help store commands' commands "$@"
}
(( $+functions[_pasejo__help__store__init_commands] )) ||
_pasejo__help__store__init_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo help store init commands' commands "$@"
}
(( $+functions[_pasejo__identity_commands] )) ||
_pasejo__identity_commands() {
    local commands; commands=(
'add:Adds an identity' /
'remove:Remove an identity' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo identity commands' commands "$@"
}
(( $+functions[_pasejo__identity__add_commands] )) ||
_pasejo__identity__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo identity add commands' commands "$@"
}
(( $+functions[_pasejo__identity__help_commands] )) ||
_pasejo__identity__help_commands() {
    local commands; commands=(
'add:Adds an identity' /
'remove:Remove an identity' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo identity help commands' commands "$@"
}
(( $+functions[_pasejo__identity__help__add_commands] )) ||
_pasejo__identity__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo identity help add commands' commands "$@"
}
(( $+functions[_pasejo__identity__help__help_commands] )) ||
_pasejo__identity__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo identity help help commands' commands "$@"
}
(( $+functions[_pasejo__identity__help__remove_commands] )) ||
_pasejo__identity__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo identity help remove commands' commands "$@"
}
(( $+functions[_pasejo__identity__remove_commands] )) ||
_pasejo__identity__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo identity remove commands' commands "$@"
}
(( $+functions[_pasejo__recipient_commands] )) ||
_pasejo__recipient_commands() {
    local commands; commands=(
'add:Adds a recipient' /
'remove:Remove a recipient' /
'inherit:Removes the recipients of a folder or secret so that it inherits its recipients from its parent' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo recipient commands' commands "$@"
}
(( $+functions[_pasejo__recipient__add_commands] )) ||
_pasejo__recipient__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient add commands' commands "$@"
}
(( $+functions[_pasejo__recipient__help_commands] )) ||
_pasejo__recipient__help_commands() {
    local commands; commands=(
'add:Adds a recipient' /
'remove:Remove a recipient' /
'inherit:Removes the recipients of a folder or secret so that it inherits its recipients from its parent' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo recipient help commands' commands "$@"
}
(( $+functions[_pasejo__recipient__help__add_commands] )) ||
_pasejo__recipient__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient help add commands' commands "$@"
}
(( $+functions[_pasejo__recipient__help__help_commands] )) ||
_pasejo__recipient__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient help help commands' commands "$@"
}
(( $+functions[_pasejo__recipient__help__inherit_commands] )) ||
_pasejo__recipient__help__inherit_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient help inherit commands' commands "$@"
}
(( $+functions[_pasejo__recipient__help__remove_commands] )) ||
_pasejo__recipient__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient help remove commands' commands "$@"
}
(( $+functions[_pasejo__recipient__inherit_commands] )) ||
_pasejo__recipient__inherit_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient inherit commands' commands "$@"
}
(( $+functions[_pasejo__recipient__remove_commands] )) ||
_pasejo__recipient__remove_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo recipient remove commands' commands "$@"
}
(( $+functions[_pasejo__store_commands] )) ||
_pasejo__store_commands() {
    local commands; commands=(
'init:Initialize a new store' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo store commands' commands "$@"
}
(( $+functions[_pasejo__store__help_commands] )) ||
_pasejo__store__help_commands() {
    local commands; commands=(
'init:Initialize a new store' /
'help:Print this message or the help of the given subcommand(s)' /
    )
    _describe -t commands 'pasejo store help commands' commands "$@"
}
(( $+functions[_pasejo__store__help__help_commands] )) ||
_pasejo__store__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo store help help commands' commands "$@"
}
(( $+functions[_pasejo__store__help__init_commands] )) ||
_pasejo__store__help__init_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo store help init commands' commands "$@"
}
(( $+functions[_pasejo__store__init_commands] )) ||
_pasejo__store__init_commands() {
    local commands; commands=()
    _describe -t commands 'pasejo store init commands' commands "$@"
}

if [ "$funcstack[1]" = "_pasejo" ]; then
    _pasejo "$@"
else
    compdef _pasejo pasejo
fi

```
