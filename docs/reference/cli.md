# Command line

`tunneler` runs the daemon and controls it. The same program does both, so there is nothing else to install.

```sh
tunneler                  # start the daemon, or ask the running one to show its window
tunneler up dev           # start every tunnel tagged "dev"
tunneler status           # what is running
tunneler down --all       # stop every tunnel
tunneler quit             # stop the daemon
```

`up` and `open` start the daemon in the background when it is not running. The other commands that talk to the daemon exit with code 3 when it is not running. Every command takes `--help` for its usage and examples.

## Choosing tunnels

Tunnels have no name. `up`, `down`, `open` and `check` select them one of three ways:

| You write | It selects |
|---|---|
| `3333` | The tunnel listening on port 3333. When several hosts have one, write `host:port`. |
| `nova:3333` | The tunnel of host `nova` listening on port 3333. |
| `dev` | Every tunnel tagged `dev`. Case is ignored. |

A tag cannot be made only of digits and cannot contain `:`, so the three forms never clash. `--tag` on `up` and `down` reads every target as a tag. `open` and `check` need a target that resolves to one tunnel: a tag works when only one tunnel carries it.

## Global options

These go with every command. Each has an environment variable.

| Option | Variable | Meaning |
|---|---|---|
| `--config <PATH>` | `TUNNELER_CONFIG` | The tunnels file. |
| `--state <PATH>` | `TUNNELER_STATE` | The file that remembers which tunnels run. |
| `--ssh-config <PATH>` | `TUNNELER_SSH_CONFIG` | Read this `ssh_config` instead of `~/.ssh/config` and the system one, like `ssh -F`. |
| `--socket <PATH>` | `TUNNELER_SOCKET` | The socket or named pipe the daemon listens on. |
| `--no-tray` | `TUNNELER_NO_TRAY` | Do not show the tray icon. |
| `--color <WHEN>` | | `auto` (the default), `always` or `never`. |
| `-V`, `--version` | | Print the version. |

## Commands

### `tunneler`

With no command, starts the daemon and its tray icon and opens the window, or asks the daemon that already runs to show its window.

### `tunneler daemon`

Runs the daemon in the foreground. Logs go to daily files, and to stderr when it is a terminal. You rarely need it, since `tunneler up` starts the daemon for you. Set `TUNNELER_LOG` to a filter such as `debug` for more detail.

### `tunneler up <TARGET>...`

Starts tunnels, and starts the daemon first when needed. It waits until each tunnel is active, and asks for a passphrase, a password or a host key confirmation when it is run in a terminal.

| Option | Meaning |
|---|---|
| `--tag` | Read the targets as tags only. |
| `--no-wait` | Return once the start is requested. |
| `--timeout <SECONDS>` | How long to wait. The default is 30. |
| `-L <SPEC>` | Start a temporary local forward. |
| `-R <SPEC>` | Start a temporary remote forward. |
| `-D <SPEC>` | Start a temporary SOCKS proxy. |

```sh
tunneler up 3333              # the tunnel listening on 3333
tunneler up nova:3333         # the same, naming the host
tunneler up dev               # every tunnel tagged dev
tunneler up --no-wait dev     # ask and return at once
tunneler up --timeout 60 dev  # wait up to a minute
```

With `-L`, `-R` or `-D` the only argument is the `ssh_config` host, and the forward uses `ssh` syntax:

```sh
tunneler up nova -L 5000                      # local 5000 to port 5000 of nova
tunneler up nova -L 5000:db.internal:5432     # local 5000 to db.internal:5432 via nova
tunneler up nova -R 9000:localhost:3000       # nova:9000 to local port 3000
tunneler up nova -D 1080                      # SOCKS proxy on local port 1080 via nova
```

`-L` and `-R` take `[bind_address:]port[:host:hostport]`, `-D` takes `[bind_address:]port`. A bare port stands for the same port on the host's loopback. A temporary tunnel is never saved, and `tunneler down` or quitting the daemon removes it. When a saved tunnel already makes the same forward, that one is started instead.

### `tunneler down [TARGET]...`

Stops tunnels. Use `--all` for every tunnel and `--tag` to read targets as tags.

```sh
tunneler down 3333
tunneler down dev
tunneler down --all
```

### `tunneler status`

Shows the daemon's version and every tunnel with its state, one per line.

```
Tunneler daemon 0.1.0: 0 of 3 tunnels active
↻ reconnecting (attempt 2)  3333 → nova:3333  [dev, web]
↻ reconnecting (attempt 2)  5432 → db.internal:5432 via nova  [dev]
○ stopped  1080 → SOCKS via nova
```

With `--json` it prints one line of JSON, `{"daemonVersion": …, "tunnels": […], "settings": {…}, "tray": {…}}`, in camelCase, with the state, target health and traffic of each tunnel. Health times are in UTC.

### `tunneler list`

Shows the configured tunnels. With `--json` it prints `{"tunnels": […]}`.

### `tunneler open <TARGET>`

Opens a local tunnel, and starts it first when stopped. A tunnel opens with its saved application, else the one suggested for its target port, else the browser. `--with <APP>` takes `browser` or an application identifier instead.

```sh
tunneler open 3333
tunneler open nova:3333
tunneler open 5999 --with browser
tunneler open 5999 --with tigervnc
```

See [Opening tunnels with an application](../guide/applications). Exit code 8 means the tunnel did not become ready or the program could not be launched, and 9 means `--with` named an application that is not available: the message lists the identifiers that are.

### `tunneler check <TARGET>`

Tries the target of a running local or remote tunnel once, and updates the target health that `status` shows. It opens one connection and closes it at once. Some services count that as a failed login, so do not run it in a loop. It exits with 0 when the target answers and 7 when it does not.

```sh
tunneler check 3333
```

### `tunneler quit`

Stops the daemon. When tunnels are active it asks first. `-y`, `--yes` stops them without asking.

### `tunneler purge`

Removes everything Tunneler left on the machine except the programs. See [Updates and uninstall](../guide/updates-and-uninstall#tunneler-purge).

| Option | Meaning |
|---|---|
| `--dry-run` | List what would be removed and remove nothing. |
| `-y`, `--yes` | Do not ask. Required without a terminal, otherwise the exit code is 5. |
| `--keep-config` | Keep `tunnels.toml`, `state.toml` and the stored passphrases. |

## Output

- `--json` is on `status` and `list`. The output is one line, meant for scripts.
- Colour is used when the output is a terminal that supports it and `NO_COLOR` is not set. `NO_COLOR` set to a non-empty value turns it off, and `--color never` does too. `--color always` forces it.
- Nothing is carried by colour alone: states are text and a symbol.
- Prompts for a passphrase, a password or a host key appear only when the command runs in a terminal.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | Error. The message on stderr says what. |
| 2 | Invalid usage. |
| 3 | The daemon is not running. |
| 4 | Some tunnels failed to start or are blocked by another program (`up`). A tunnel provided by an external `ssh` counts as started. |
| 5 | A confirmation is needed but cannot be asked: not a terminal, pass `--yes` (`quit`, `purge`). |
| 6 | Timed out waiting. |
| 7 | The target of the tunnel does not answer (`check`). |
| 8 | The tunnel did not become ready, or the browser or application could not be launched (`open`). |
| 9 | The application given with `--with` is not available (`open`). |

## Environment variables

| Variable | Meaning |
|---|---|
| `NO_COLOR` | Turn colour off. |
| `TUNNELER_LOG` | Log filter for the daemon and the commands, such as `debug`. |
| `TUNNELER_SOCKET`, `TUNNELER_CONFIG`, `TUNNELER_STATE`, `TUNNELER_SSH_CONFIG`, `TUNNELER_NO_TRAY` | Same as the matching option. |
| `TUNNELER_UI_BINARY` | The `tunneler-ui` program the daemon launches. |
