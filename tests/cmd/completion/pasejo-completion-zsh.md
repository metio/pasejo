```
$ COMPLETE=zsh pasejo
#compdef pasejo
function _clap_dynamic_completer_pasejo() {
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
    local _CLAP_IFS=$'/n'

    local completions=("${(@f)$( /
        _CLAP_IFS="$_CLAP_IFS" /
        _CLAP_COMPLETE_INDEX="$_CLAP_COMPLETE_INDEX" /
        COMPLETE="zsh" /
        [CWD]/target/debug/pasejo -- "${words[@]}" 2>/dev/null /
    )}")

    if [[ -n $completions ]]; then
        _describe 'values' completions
    fi
}

compdef _clap_dynamic_completer_pasejo pasejo

```
