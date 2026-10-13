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
| `g` | Open Unified Logs |
| `p` | Pause/resume visible Unified Logs without stopping acquisition |
| `F` | Toggle follow mode in Unified Logs |
| `f` | Edit the next Unified Logs filter field |
| `/` | Search Unified Logs incrementally |
| `n` / `N` | Next/previous Unified Logs match |
| `j` / `k` | Move through Unified Logs |
| `g` / `G` | Jump to start/end in Unified Logs |
| `E` | Export all retained logs as TXT |
| `J` | Export filtered logs as JSONL |
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

The **Logs** tab keeps a bounded in-memory history. `f` cycles through container, Compose project/service, severity, time and text fields; Enter applies the field and an empty value clears it. Pause does not stop acquisition. Cross-container ordering is approximate and does not guarantee causal ordering. Exported TXT/JSONL files may contain secrets already present in application logs. Export is treated as a local read-only operation, so it remains available in read-only Docker contexts; an existing path is rejected rather than overwritten.

Exec uses the standard crossterm/xterm key protocol. Terminals using proprietary enhanced-key protocols, kitty graphics, or sixel output may have reduced compatibility.

Interactive input is queued in a bounded 256 KiB FIFO. If a paste exceeds the available queue capacity, it is rejected with a warning rather than silently dropping bytes.
