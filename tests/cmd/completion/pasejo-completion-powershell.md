```console
$ pasejo completion powershell

using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'pasejo' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'pasejo'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'pasejo' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Optional name of store to use. Defaults to the first store defined in the local user configuration')
            [CompletionResult]::new('--store', '--store', [CompletionResultType]::ParameterName, 'Optional name of store to use. Defaults to the first store defined in the local user configuration')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('identity', 'identity', [CompletionResultType]::ParameterValue, 'Manage identities')
            [CompletionResult]::new('recipient', 'recipient', [CompletionResultType]::ParameterValue, 'Manage recipients')
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Manage stores')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;completion' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;identity' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds an identity')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an identity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;identity;add' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'The path to the identity file')
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'The path to the identity file')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;identity;remove' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'The path to the identity file')
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'The path to the identity file')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;identity;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds an identity')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an identity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;identity;help;add' {
            break
        }
        'pasejo;identity;help;remove' {
            break
        }
        'pasejo;identity;help;help' {
            break
        }
        'pasejo;recipient' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds a recipient')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a recipient')
            [CompletionResult]::new('inherit', 'inherit', [CompletionResultType]::ParameterValue, 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;recipient;add' {
            [CompletionResult]::new('-k', '-k', [CompletionResultType]::ParameterName, 'The public key of the new recipient')
            [CompletionResult]::new('--public-key', '--public-key', [CompletionResultType]::ParameterName, 'The public key of the new recipient')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'The name of the new recipient')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'The name of the new recipient')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should be readable by the given recipient')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should be readable by the given recipient')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;recipient;remove' {
            [CompletionResult]::new('-k', '-k', [CompletionResultType]::ParameterName, 'The public key of the recipient to remove')
            [CompletionResult]::new('--public-key', '--public-key', [CompletionResultType]::ParameterName, 'The public key of the recipient to remove')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should no longer be readable by the given recipient')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should no longer be readable by the given recipient')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;recipient;inherit' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should inherit its recipients from its parent')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'The path to a folder or secret that should inherit its recipients from its parent')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;recipient;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds a recipient')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a recipient')
            [CompletionResult]::new('inherit', 'inherit', [CompletionResultType]::ParameterValue, 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;recipient;help;add' {
            break
        }
        'pasejo;recipient;help;remove' {
            break
        }
        'pasejo;recipient;help;inherit' {
            break
        }
        'pasejo;recipient;help;help' {
            break
        }
        'pasejo;store' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new store')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;store;init' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The path on your local system for the new store')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'The path on your local system for the new store')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'The alias for the new store')
            [CompletionResult]::new('--alias', '--alias', [CompletionResultType]::ParameterName, 'The alias for the new store')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'The version control system to use')
            [CompletionResult]::new('--vcs', '--vcs', [CompletionResultType]::ParameterName, 'The version control system to use')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pasejo;store;help' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new store')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;store;help;init' {
            break
        }
        'pasejo;store;help;help' {
            break
        }
        'pasejo;help' {
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('identity', 'identity', [CompletionResultType]::ParameterValue, 'Manage identities')
            [CompletionResult]::new('recipient', 'recipient', [CompletionResultType]::ParameterValue, 'Manage recipients')
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Manage stores')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pasejo;help;completion' {
            break
        }
        'pasejo;help;identity' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds an identity')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an identity')
            break
        }
        'pasejo;help;identity;add' {
            break
        }
        'pasejo;help;identity;remove' {
            break
        }
        'pasejo;help;recipient' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Adds a recipient')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a recipient')
            [CompletionResult]::new('inherit', 'inherit', [CompletionResultType]::ParameterValue, 'Removes the recipients of a folder or secret so that it inherits its recipients from its parent')
            break
        }
        'pasejo;help;recipient;add' {
            break
        }
        'pasejo;help;recipient;remove' {
            break
        }
        'pasejo;help;recipient;inherit' {
            break
        }
        'pasejo;help;store' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new store')
            break
        }
        'pasejo;help;store;init' {
            break
        }
        'pasejo;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}

```
