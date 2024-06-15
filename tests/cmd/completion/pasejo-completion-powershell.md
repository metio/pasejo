```
$ COMPLETE=powershell pasejo

Register-ArgumentCompleter -Native -CommandName pasejo -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $prev = $env:COMPLETE;
    $env:COMPLETE = "powershell";
    $results = Invoke-Expression @"
& [CWD]/target/debug/pasejo -- $commandAst
"@;
    if ($null -eq $prev) {
        Remove-Item Env:/COMPLETE;
    } else {
        $env:COMPLETE = $prev;
    }
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
