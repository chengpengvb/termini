pub const IDENTITY_SECTION_SEPARATOR: &str = "---";
pub const READ_IDENTITY_COMMAND: &str = r#"sh -c '
if command -v getent >/dev/null 2>&1; then
  getent passwd | awk -F: "{print \$3\":\"\$1}"
  echo "---"
  getent group  | awk -F: "{print \$3\":\"\$1}"
else
  awk -F: "{print \$3\":\"\$1}" /etc/passwd
  echo "---"
  awk -F: "{print \$3\":\"\$1}" /etc/group
fi
'"#;

pub const READ_HISTORY_COMMAND: &str = r#"sh -c '
for f in ${HISTFILE:+"$HISTFILE"} "$HOME/.bash_history" "$HOME/.zsh_history"; do
  [ -n "$f" ] && [ -f "$f" ] || continue
  sed -e "s/^: [0-9][0-9]*:[0-9][0-9]*;//" "$f"
done | tail -n 350
'"#;
