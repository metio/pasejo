```console
$ pasejo completion fish
# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_pasejo_global_optspecs
	string join /n s/store= h/help
end

function __fish_pasejo_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_pasejo_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_pasejo_using_subcommand
	set -l cmd (__fish_pasejo_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c pasejo -n "__fish_pasejo_needs_command" -s s -l store -d 'Optional name of store to use. Defaults to the first store defined in the local user configuration' -r
complete -c pasejo -n "__fish_pasejo_needs_command" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_needs_command" -f -a "completion" -d 'Generate shell completions'
complete -c pasejo -n "__fish_pasejo_needs_command" -f -a "identity" -d 'Manage identities'
complete -c pasejo -n "__fish_pasejo_needs_command" -f -a "recipient" -d 'Manage recipients'
complete -c pasejo -n "__fish_pasejo_needs_command" -f -a "store" -d 'Manage stores'
complete -c pasejo -n "__fish_pasejo_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand completion" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and not __fish_seen_subcommand_from add remove help" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and not __fish_seen_subcommand_from add remove help" -f -a "add" -d 'Adds an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and not __fish_seen_subcommand_from add remove help" -f -a "remove" -d 'Remove an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and not __fish_seen_subcommand_from add remove help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from add" -s f -l file -d 'The path to the identity file' -r -F
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from remove" -s f -l file -d 'The path to the identity file' -r -F
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from help" -f -a "add" -d 'Adds an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand identity; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and not __fish_seen_subcommand_from add remove inherit help" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and not __fish_seen_subcommand_from add remove inherit help" -f -a "add" -d 'Adds a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and not __fish_seen_subcommand_from add remove inherit help" -f -a "remove" -d 'Remove a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and not __fish_seen_subcommand_from add remove inherit help" -f -a "inherit" -d 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and not __fish_seen_subcommand_from add remove inherit help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from add" -s k -l public-key -d 'The public key of the new recipient' -r
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from add" -s n -l name -d 'The name of the new recipient' -r
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from add" -s p -l path -d 'The path to a folder or secret that should be readable by the given recipient' -r -F
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from remove" -s k -l public-key -d 'The public key of the recipient to remove' -r
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from remove" -s p -l path -d 'The path to a folder or secret that should no longer be readable by the given recipient' -r -F
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from inherit" -s p -l path -d 'The path to a folder or secret that should inherit its recipients from its parent' -r -F
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from inherit" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from help" -f -a "add" -d 'Adds a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from help" -f -a "inherit" -d 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
complete -c pasejo -n "__fish_pasejo_using_subcommand recipient; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and not __fish_seen_subcommand_from init help" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and not __fish_seen_subcommand_from init help" -f -a "init" -d 'Initialize a new store'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and not __fish_seen_subcommand_from init help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from init" -s p -l path -d 'The path on your local system for the new store' -r -f -a "(__fish_complete_directories)"
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from init" -s a -l alias -d 'The alias for the new store' -r
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from init" -s v -l vcs -d 'The version control system to use' -r -f -a "{none/t'',git/t'',mercurial/t''}"
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from init" -s h -l help -d 'Print help'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from help" -f -a "init" -d 'Initialize a new store'
complete -c pasejo -n "__fish_pasejo_using_subcommand store; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and not __fish_seen_subcommand_from completion identity recipient store help" -f -a "completion" -d 'Generate shell completions'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and not __fish_seen_subcommand_from completion identity recipient store help" -f -a "identity" -d 'Manage identities'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and not __fish_seen_subcommand_from completion identity recipient store help" -f -a "recipient" -d 'Manage recipients'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and not __fish_seen_subcommand_from completion identity recipient store help" -f -a "store" -d 'Manage stores'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and not __fish_seen_subcommand_from completion identity recipient store help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from identity" -f -a "add" -d 'Adds an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from identity" -f -a "remove" -d 'Remove an identity'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from recipient" -f -a "add" -d 'Adds a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from recipient" -f -a "remove" -d 'Remove a recipient'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from recipient" -f -a "inherit" -d 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent'
complete -c pasejo -n "__fish_pasejo_using_subcommand help; and __fish_seen_subcommand_from store" -f -a "init" -d 'Initialize a new store'

```
