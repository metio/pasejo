```console
$ COMPLETE=elvish pasejo

set edit:completion:arg-completer[pasejo] = { |@words|
    var index = (count $words)
    set index = (- $index 1)

    put (env _CLAP_IFS="/n" _CLAP_COMPLETE_INDEX=(to-string $index) COMPLETE="elvish" [CWD]/target/debug/pasejo -- $@words) | to-lines
}


```
