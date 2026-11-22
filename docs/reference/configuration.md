# Configuration

Your tunnels and settings live in one TOML file, `tunnels.toml`. The window edits it, and you can edit it by hand: Tunneler reloads it when it changes. When the window rewrites the file it keeps your comments and formatting. Writes are atomic.

A file with a syntax error, an unknown key or a broken rule is refused, and Tunneler says what is wrong. Every key below is optional, apart from the ones a tunnel needs.

## File locations

| File | Linux | Windows | Portable |
|---|---|---|---|
| `tunnels.toml` | `~/.config/tunneler/` | `%APPDATA%\tunneler\` | next to the programs |
| `state.toml` | `~/.local/state/tunneler/` | `%LOCALAPPDATA%\tunneler\` | `data/` |
| Logs | `~/.local/share/tunneler/logs/` | `%LOCALAPPDATA%\tunneler\logs\` | `data/logs/` |
| Window web data | `~/.local/share/com.minimalstuff.tunneler/` | `%LOCALAPPDATA%\com.minimalstuff.tunneler\` | `data/webview/` |

On Linux the first three honour `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_DATA_HOME`. Logs are daily files, and the daemon keeps 7. Set `TUNNELER_LOG` to a filter such as `debug` for more detail. "Open the configuration folder" in the settings opens the folder of `tunnels.toml`.

Portable mode is on while an empty file named `portable` sits next to the programs. The Windows zip ships it; on Linux you create it yourself. See [Updates and uninstall](../guide/updates-and-uninstall#portable-mode).

### Choosing other files

| Option | Environment variable | What it sets |
|---|---|---|
| `--config <PATH>` | `TUNNELER_CONFIG` | The tunnels file. |
| `--state <PATH>` | `TUNNELER_STATE` | The state file. |
| `--ssh-config <PATH>` | `TUNNELER_SSH_CONFIG` | The `ssh_config` to read instead of `~/.ssh/config` and the system one, like `ssh -F`. |
| `--socket <PATH>` | `TUNNELER_SOCKET` | The socket, or named pipe on Windows, the daemon listens on. |

They win over the locations above, portable or not. They apply to every command, and a daemon that a command starts gets the same ones.

## Example

```toml
version = 1                         # the format version; 1 when absent

[settings]
default_bind_address = "127.0.0.1"  # local and dynamic tunnels listen here
default_remote_bind_address = "localhost"  # remote tunnels listen here, on the server
keepalive_interval_s = 15           # 0 turns keepalive off
keepalive_max = 3                   # unanswered probes before the session is lost
session_idle_grace_s = 30           # an unused session stays open this long
reconnect_initial_delay_ms = 1000
reconnect_max_delay_ms = 60000
restore_on_launch = true            # restart the tunnels that were running at the last exit
start_on_launch_tags = ["dev"]      # tags started when the daemon launches
launch_at_login = false
use_os_keyring = false              # let passphrases be remembered in the system keyring
x11_display = ""                    # empty: $DISPLAY, or 127.0.0.1:0.0 on Windows
x11_server_start = "ask"            # ask | always | never
open_host = "localhost"             # localhost | ip
open_default = "suggested"          # suggested | browser
standby = false                     # local and dynamic tunnels wait in standby by default
standby_idle_s = 300                # back to standby after this long without a connection
standby_notify = "visual"           # none | visual | sound | both
standby_sound = "pluck"             # system | chime | marimba | pluck
standby_volume = 50                 # 5 to 100 (%), for chime, marimba and pluck
language = "system"                 # system | en | fr
check_for_updates = true            # look for a newer release when the window opens
devslot_persist = false             # devslot discovery saves the tunnels instead of keeping them temporary
devslot_stale = "ask"               # ask | delete: a saved devslot tunnel whose port is gone

[[tunnels]]
id = "01JA0000000000000000000000"   # a ULID; Tunneler adds it when it is missing
host = "nova"                       # an alias of ~/.ssh/config
kind = "local"                      # local | remote | dynamic
listen_port = 3333
# listen_address = "0.0.0.0"        # instead of the default bind address
# target_host = "127.0.0.1"         # where the server connects to; default 127.0.0.1
# target_port = 3333                # default: listen_port
tags = ["dev", "web"]
# standby = true                    # instead of settings.standby
# open = { scheme = "https", path = "/admin", host = "ip", with = "tigervnc" }

[[tunnels]]
host = "nova"
kind = "local"
listen_port = 5432
target_host = "db.internal"         # reached from nova
tags = ["dev"]

[[tunnels]]
host = "nova"
kind = "dynamic"                    # a SOCKS5 proxy, no target
listen_port = 1080

[[openers]]                         # applications you added; built-in ones are not listed here
id = "dbeaver"                      # lowercase letters, digits, dashes; "browser" is reserved
name = "DBeaver"
command = "dbeaver"
args = ["-con", "driver=postgresql|host={host}|port={port}"]
ports = [5432]                      # target ports this application is suggested for

[hosts.nova]                        # app-only settings per ssh_config alias
x11_display = "198.51.100.135:0.0"

[[x11_launchers]]
host = "nova"
name = "VS Code"
command = "code --wait"
```

## `[settings]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `default_bind_address` | string | `"127.0.0.1"` | Address local and dynamic tunnels listen on. Must not be empty. |
| `default_remote_bind_address` | string | `"localhost"` | Address remote tunnels ask the server to listen on. Must not be empty. |
| `keepalive_interval_s` | integer | `15` | Seconds between keepalive probes. `0` turns keepalive off. A `ServerAliveInterval` in `ssh_config` wins. |
| `keepalive_max` | integer | `3` | Unanswered probes before the session is considered lost. Must be at least 1 while keepalive is on. A `ServerAliveCountMax` in `ssh_config` wins. |
| `session_idle_grace_s` | integer | `30` | Seconds an SSH session stays open after its last tunnel stops. |
| `reconnect_initial_delay_ms` | integer | `1000` | First wait before reconnecting. Greater than 0 and not above the maximum. |
| `reconnect_max_delay_ms` | integer | `60000` | Upper bound of the wait, which grows exponentially with some jitter. |
| `restore_on_launch` | boolean | `true` | Start again the tunnels that were running at the last exit. |
| `start_on_launch_tags` | array of strings | `[]` | Tags whose tunnels start when the daemon launches. |
| `launch_at_login` | boolean | `false` | Start the daemon when you log in. |
| `use_os_keyring` | boolean | `false` | Offer to remember key passphrases in the system keyring (Credential Manager on Windows, Secret Service on Linux). When off, a remembered passphrase lasts until Tunneler quits. |
| `x11_display` | string | `""` | Local X display. Empty means `$DISPLAY`, or `127.0.0.1:0.0` on Windows. |
| `x11_server_start` | `"ask"`, `"always"` or `"never"` | `"ask"` | What to do when no X server answers on the local display before X11 or a launcher starts. `ask`: offer to start the installed X server (VcXsrv or Xming), `always`: start it, `never`: only warn. X11 starts in every case. |
| `open_host` | `"localhost"` or `"ip"` | `"localhost"` | Name used to open a tunnel that listens on a loopback or wildcard address: `localhost`, or `127.0.0.1` (`[::1]` for IPv6). |
| `open_default` | `"suggested"` or `"browser"` | `"suggested"` | What a tunnel without `open.with` opens with. `suggested`: the application suggested for its target port, else the browser. `browser`: always the browser. A saved `open.with` and an explicit request (`--with`, "Open with…") still win. |
| `standby` | boolean | `false` | Put local and dynamic tunnels in standby unless they say otherwise. |
| `standby_idle_s` | integer | `300` | Seconds without a connection before a tunnel goes back to standby. Greater than 0. |
| `standby_notify` | `"none"`, `"visual"`, `"sound"` or `"both"` | `"visual"` | How a wake-up from standby, or its failure, is signalled. |
| `standby_sound` | `"system"`, `"chime"`, `"marimba"` or `"pluck"` | `"pluck"` | The sound played in `sound` and `both` modes. |
| `standby_volume` | integer | `50` | The volume of the `chime`, `marimba` and `pluck` sounds, in percent, from 5 to 100. The `system` sound ignores it and follows the system volume. |
| `language` | `"system"`, `"en"` or `"fr"` | `"system"` | Language of the window, the tray and the notifications. `system` follows the operating system. |
| `check_for_updates` | boolean | `true` | Look for a newer release when the window opens. |
| `devslot_persist` | boolean | `false` | Initial value of "Keep these tunnels" in [devslot discovery](../guide/devslot): save the tunnels instead of keeping them temporary. A host can override it. |
| `devslot_stale` | `"ask"` or `"delete"` | `"ask"` | What a discovery does with a saved devslot tunnel whose port is no longer listed. `ask`: offer to delete it. `delete`: delete it. It never deletes anything unasked when the listing is empty or when every saved devslot tunnel of the host would go. |

## `[[tunnels]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | ULID string | generated | Stable identifier. Two tunnels cannot share one. When missing, Tunneler adds it to the file. |
| `host` | string | required | Alias from `ssh_config`. Not empty, and it must follow the rules of [Host names](#host-names). |
| `kind` | `"local"`, `"remote"` or `"dynamic"` | required | The direction of the forward, as `-L`, `-R` and `-D`. |
| `listen_port` | integer | required | Port to listen on, 1 to 65535. Local: on this computer. Remote: on the server. |
| `listen_address` | string | `default_bind_address`, or `default_remote_bind_address` for remote | Address to listen on. A host name cannot be opened with "Open". |
| `target_host` | string | `"127.0.0.1"` | Where the traffic goes. Local: seen from the server. Remote: seen from this computer. Not allowed on a dynamic tunnel. |
| `target_port` | integer | `listen_port` | Port the traffic goes to, 1 to 65535. Not allowed on a dynamic tunnel. |
| `tags` | array of strings | `[]` | Labels. No whitespace, comma or colon; not digits only; no repeats in one tunnel. |
| `standby` | boolean | `settings.standby` | `true` or `false` for this tunnel. Ignored on a remote tunnel. |
| `open` | table | `{}` | How the tunnel is opened. |

Two local or dynamic tunnels cannot listen on the same address and port. Two remote tunnels of the same host cannot either.

### Host names

A host alias is passed to the `ssh_config` evaluation, where `%h` and other tokens are expanded, so Tunneler applies the rule of OpenSSH 9.6 (the fix for CVE-2023-51385). An alias may only contain letters and digits (any script) and the characters `.` `_` `-` `:` `@` `[` `]`. It may not start with `-` and is at most 255 bytes. Anything else, such as whitespace, quotes, `$`, `;`, `&`, `|`, `<`, `>`, parentheses, braces or a control character, is refused. `%` is refused too, because `cmd` expands `%VAR%` on Windows.

- Creating or editing a tunnel, starting a temporary tunnel and importing a file refuse such a host.
- The import from `ssh_config` and from pasted `ssh -L` scripts skips such a candidate and shows a warning.
- A hand-edited `tunnels.toml` with such a host still loads, but the tunnel cannot connect: the alias is never looked up in `ssh_config` and no connection is made, and the error says why.

### `open`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `scheme` | `"http"` or `"https"` | `"http"` | Scheme of the URL opened in the browser. |
| `path` | string | `"/"` | Path of that URL. Starts with `/`, not `//`, with no whitespace or control character. |
| `host` | `"localhost"` or `"ip"` | `settings.open_host` | Name used for a loopback or wildcard listen address. A tunnel listening on a specific IP opens that IP. A remote host name is never used (see `host_name` for the `.localhost` exception). |
| `host_name` | string | none | A name of the form `<label>.localhost`, with lowercase letters, digits and dashes in each label. Used instead of `localhost` or the IP in the URL opened in the browser, when the tunnel listens on a loopback or wildcard address. Ignored on a specific IP. Programs still get the usual host in `{host}`. Any other name is refused. Set by [devslot discovery](../guide/devslot). |
| `with` | string | automatic | `"browser"` or the identifier of an application. When absent, the application suggested for the target port (unless `settings.open_default` is `"browser"`), else the browser. |

## `[[openers]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Lowercase letters, digits and dashes. Unique. `browser` is reserved. The same id as a built-in application replaces it. |
| `name` | string | required | Label shown in the window. |
| `command` | string | required | The program, run without a shell. |
| `args` | array of strings | `[]` | One argument per entry. `{host}`, `{port}` and `{url}` are replaced; any other `{…}` is refused. |
| `ports` | array of integers | `[]` | Target ports this application is suggested for. |

See [Opening tunnels with an application](../guide/applications).

## `[hosts.<alias>]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `x11_display` | string | none | Local X display used for this host, before `settings.x11_display` and `$DISPLAY`. |
| `devslot_path` | string | `"ds"` | Command that runs devslot on this host, for [devslot discovery](../guide/devslot). Not empty, at most 4096 bytes, no control character. A leading `~/` is expanded by the remote shell. Set it when `ds` is not in the `PATH` of a non-interactive SSH shell. |
| `devslot_persist` | boolean | `settings.devslot_persist` | Overrides the setting for this host. |

The `devslot_*` keys of a host are edited by hand: the window does not write them.

## `[[x11_launchers]]`

| Key | Type | Meaning |
|---|---|---|
| `host` | string | The alias the command runs on. |
| `name` | string | Label shown in the window. Unique per host. |
| `command` | string | The command line. |

All three are required and not empty. See [X11 forwarding](../guide/x11).

## Export file

`tunneler export` and the window's export write a TOML file that [another computer can import](../guide/export-import). It holds tunnels only.

```toml
# Tunneler export (format version 1)
format = "tunneler-export"
version = 1

[[tunnels]]
host = "nova"
kind = "local"
listen_port = 3333
tags = ["dev"]

[tunnels.open]
path = "/admin"
```

| Key | Meaning |
|---|---|
| `format` | Always `"tunneler-export"`. |
| `version` | The version of the export format, `1`. A file with a greater version is refused as coming from a newer Tunneler. |
| `[[tunnels]]` | The same keys as [`[[tunnels]]`](#tunnels) above, without `id`. Keys that are not set are left out. |

The tunnels come in the order of `tunnels.toml`. The file has no `[settings]`, `[[openers]]`, `[hosts.<alias>]` or `[[x11_launchers]]`, and nothing from `state.toml`. An unknown key is refused.

The import also reads a `tunnels.toml` (a file with no `format` key): it takes its `[[tunnels]]`, ignores every other table and replaces the ids. Any other `format` value is refused. A file is at most 256 KiB, with at most 1000 tunnels, and a string value with a control or invisible direction character makes the import refuse the whole file.

## `state.toml`

Tunneler writes `state.toml` itself: you do not edit it. It remembers which tunnels you wanted running, so that `restore_on_launch` can start them again, and which key files have a passphrase in the system keyring, so that Tunneler can forget them later. Temporary tunnels are never in it. It stays apart from `tunnels.toml` so that the file you edit by hand does not change every time you start a tunnel.
