# runic — bash integration
# Install: eval "$(runic init bash)"

# Bash's Esc is a readline prefix, so bind double-Esc instead.
_runic_pick_widget() {
  local cmd
  cmd=$(command runic pick </dev/tty 2>/dev/tty)
  if [[ -n "$cmd" ]]; then
    READLINE_LINE="$cmd"
    READLINE_POINT=${#cmd}
  fi
}
bind -x '"\e\e": _runic_pick_widget' 2>/dev/null
