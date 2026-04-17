# runic — zsh integration
# Install: eval "$(runic init zsh)"

_runic_widget() {
  if [[ -z "$BUFFER" ]]; then
    zle -I
    local cmd
    cmd=$(command runic pick </dev/tty 2>/dev/tty)
    if [[ -n "$cmd" ]]; then
      BUFFER="$cmd"
      zle accept-line
    else
      zle reset-prompt
    fi
  else
    zle vi-cmd-mode 2>/dev/null
  fi
}
zle -N _runic_widget
bindkey '\e' _runic_widget
