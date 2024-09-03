```console
$ COMPLETE=powershell pasejo

Register-ArgumentCompleter -Native -CommandName pasejo -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $results = Invoke-Expression "COMPLETE=powershell &pasejo -- $($commandAst.ToString())";
    $results | ForEach-Object {
        $split = $_.Split("`t");
        $cmd = $split[0];

        if ($split.Length -eq 2) {
            $help = $split[1];
        }
        else {
            $help = $split[0];
        }

        [System.Management.Automation.CompletionResult]::new($cmd, $cmd, 'ParameterValue', $help)
    }
};
        

```
