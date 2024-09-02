```console
$ pasejo completion elvish

use builtin;
use str;

set edit:completion:arg-completer[pasejo] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'pasejo'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'pasejo'= {
            cand -s 'Optional name of store to use. Defaults to the first store defined in the local user configuration'
            cand --store 'Optional name of store to use. Defaults to the first store defined in the local user configuration'
            cand -h 'Print help'
            cand --help 'Print help'
            cand completion 'Generate shell completions'
            cand identity 'Manage identities'
            cand recipient 'Manage recipients'
            cand store 'Manage stores'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;completion'= {
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;identity'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand add 'Adds an identity'
            cand remove 'Remove an identity'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;identity;add'= {
            cand -f 'The path to the identity file'
            cand --file 'The path to the identity file'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;identity;remove'= {
            cand -f 'The path to the identity file'
            cand --file 'The path to the identity file'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;identity;help'= {
            cand add 'Adds an identity'
            cand remove 'Remove an identity'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;identity;help;add'= {
        }
        &'pasejo;identity;help;remove'= {
        }
        &'pasejo;identity;help;help'= {
        }
        &'pasejo;recipient'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand add 'Adds a recipient'
            cand remove 'Remove a recipient'
            cand inherit 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;recipient;add'= {
            cand -k 'The public key of the new recipient'
            cand --public-key 'The public key of the new recipient'
            cand -n 'The name of the new recipient'
            cand --name 'The name of the new recipient'
            cand -p 'The path to a folder or secret that should be readable by the given recipient'
            cand --path 'The path to a folder or secret that should be readable by the given recipient'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;recipient;remove'= {
            cand -k 'The public key of the recipient to remove'
            cand --public-key 'The public key of the recipient to remove'
            cand -p 'The path to a folder or secret that should no longer be readable by the given recipient'
            cand --path 'The path to a folder or secret that should no longer be readable by the given recipient'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;recipient;inherit'= {
            cand -p 'The path to a folder or secret that should inherit its recipients from its parent'
            cand --path 'The path to a folder or secret that should inherit its recipients from its parent'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;recipient;help'= {
            cand add 'Adds a recipient'
            cand remove 'Remove a recipient'
            cand inherit 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;recipient;help;add'= {
        }
        &'pasejo;recipient;help;remove'= {
        }
        &'pasejo;recipient;help;inherit'= {
        }
        &'pasejo;recipient;help;help'= {
        }
        &'pasejo;store'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand init 'Initialize a new store'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;store;init'= {
            cand -p 'The path on your local system for the new store'
            cand --path 'The path on your local system for the new store'
            cand -a 'The alias for the new store'
            cand --alias 'The alias for the new store'
            cand -v 'The version control system to use'
            cand --vcs 'The version control system to use'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pasejo;store;help'= {
            cand init 'Initialize a new store'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;store;help;init'= {
        }
        &'pasejo;store;help;help'= {
        }
        &'pasejo;help'= {
            cand completion 'Generate shell completions'
            cand identity 'Manage identities'
            cand recipient 'Manage recipients'
            cand store 'Manage stores'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pasejo;help;completion'= {
        }
        &'pasejo;help;identity'= {
            cand add 'Adds an identity'
            cand remove 'Remove an identity'
        }
        &'pasejo;help;identity;add'= {
        }
        &'pasejo;help;identity;remove'= {
        }
        &'pasejo;help;recipient'= {
            cand add 'Adds a recipient'
            cand remove 'Remove a recipient'
            cand inherit 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
        }
        &'pasejo;help;recipient;add'= {
        }
        &'pasejo;help;recipient;remove'= {
        }
        &'pasejo;help;recipient;inherit'= {
        }
        &'pasejo;help;store'= {
            cand init 'Initialize a new store'
        }
        &'pasejo;help;store;init'= {
        }
        &'pasejo;help;help'= {
        }
    ]
    $completions[$command]
}

```
