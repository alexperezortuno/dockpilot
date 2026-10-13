# Keyboard Shortcuts

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Change focus |
| `Left` / `Right` | Change resource tab |
| `Up` / `Down` / `j` / `k` | Navigate the focused component |
| `Enter` | Execute the selected action or confirm input |
| `PageUp` / `PageDown` | Scroll output |
| `Home` / `End` | Jump to output start/end |
| `a` | Open actions for the selected resource |
| `:` | Open the command palette |
| `/` | Incrementally filter the current resource |
| `r` / `R` | Refresh / toggle auto-refresh |
| `S` | Refresh local system information |
| `x` | Cancel the active task |
| `o` / `D` / `C` | Toggle output / details / compact layout |
| `?` | Open help |
| `q` / `Esc` / `Ctrl+C` | Quit |

## Exec Mode

Open `Exec into container` from the `a` context menu or command palette on a running container.

| Key | Action |
| --- | --- |
| Characters, Enter, Backspace, Tab, arrows | Send input to the container PTY |
| `Ctrl+C` | Send interrupt to the process inside the container |
| `Ctrl+]` | Request detach; Dockpilot returns only after the local PTY detach is confirmed, without terminating the remote process |
| `Shift+PageUp` / `Shift+PageDown` | Move through bounded terminal scrollback |
| `Esc` / `Enter` after exit | Close the finished session |

In the command palette, arrows navigate and `j`/`k` are normal text input. Destructive actions always require confirmation.

Exec uses the standard crossterm/xterm key protocol. Terminals using proprietary enhanced-key protocols, kitty graphics, or sixel output may have reduced compatibility.

Interactive input is queued in a bounded 256 KiB FIFO. If a paste exceeds the available queue capacity, it is rejected with a warning rather than silently dropping bytes.
