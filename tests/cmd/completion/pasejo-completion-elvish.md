```console
$ COMPLETE=elvish pasejo

set edit:completion:arg-completer[pasejo] = { |@words|
    set E:_CLAP_IFS = "/n"

    var index = (count $words)
    set index = (- $index 1)
    set E:_CLAP_COMPLETE_INDEX = (to-string $index)
    set E:COMPLETE = "elvish"

    put (pasejo -- $@words) | to-lines
}


```
