# Command line

`tunneler` runs the daemon and controls it. The same program does both, so there is nothing else to install.

```sh
tunneler                  # start the daemon, or ask the running one to show its window
tunneler up dev           # start every tunnel tagged "dev"
tunneler status           # what is running
tunneler down --all       # stop every tunnel
tunneler quit             # stop the daemon
```

`up`, `open` and `import` start the daemon in the background when it is not running. The other commands that talk to the daemon exit with code 3 when it is not running. Every command takes `--help` for its usage and examples.

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

### `tunneler devslot <HOST> <INSTANCE>`

Tunnels the web ports of a [devslot](../guide/devslot) instance and opens it in the browser. `HOST` is an alias of your `ssh_config` and `INSTANCE` is the name of the instance as `ds ls` prints it. The daemon is started when it is not running. If a password, a passphrase or an unknown host key has to be confirmed, you are asked in the terminal when there is one. Running it again creates nothing new.

Both arguments are 1 to 64 characters (`HOST`) or 1 to 255 (`INSTANCE`) of letters, digits, `.`, `_` and `-`, the first one a letter or a digit. Anything else is refused with exit code 2, before any connection.

| Option | Meaning |
|---|---|
| `--no-open` | Start the tunnels and leave the browser alone. |
| `--json` | Print the result as one line of JSON. |

Only web ports are tunneled, and not those below 1024 (skipped by design, not a failure). The text output gives one line per port (started, already set up, skipped with the reason, failed with the message), the stale routes it removed first, then the address opened. With `--json`, the line is an object with the keys `host`, `instance`, `started`, `existing`, `skipped`, `failed`, `removed`, `needsReview`, `appPortShared`, and, when they apply, `appUrl`, `opened` and `openError`. `started` and `existing` list `{ remotePort, listenPort, tunnelId }`, `skipped` lists `{ remotePort, reason }` with a reason of `localPortBusy`, `needsReview`, `existingTunnel` (a tunnel you configured, or one listening beyond the loopback, already forwards the port: start it yourself) or `privilegedPort`, `failed` lists `{ remotePort, message }`, and `removed` lists `{ listenPort, remotePort, isTemporary }`.

Exit codes: 0 when the instance is up, 1 for an error (unknown host, instance not listed, connection or `ds` failure), 4 when some web ports failed or were skipped because the local port is busy, a stale tunnel needs your decision or a tunnel you configured already forwards the port, 6 when the command on the host timed out, 8 when the browser could not be opened.

```sh
tunneler devslot vega web-shop-098a
tunneler devslot vega web-shop-098a --no-open
tunneler devslot vega web-shop-098a --json
```

### `tunneler export`

Writes your saved tunnels to a file you can [import on another computer](../guide/export-import). The daemon must be running: otherwise the command exits with code 3.

| Option | Meaning |
|---|---|
| `--host <HOST>` | Only the tunnels of this host. Repeat it for several hosts. |
| `--tag <TAG>` | Only the tunnels with this tag. Repeat it for several tags. |
| `-o`, `--output <FILE>` | Write to this file instead of the standard output. An existing file or symbolic link is refused. On Unix the new file is readable by you only (mode 0600). |
| `--force` | Replace the file given with `-o` if it exists. Needs `-o`. |
| `--json` | Print the result as one line of JSON. |

A tunnel is exported when it matches any of the `--host` and `--tag` filters. Without a filter, every saved tunnel is. Temporary tunnels are never exported. An export larger than 256 KiB (the import limit) is refused with `the export is larger than 256 KiB; filter by host or tag`, and the hint `pass --host or --tag to export a part of the tunnels, then the others in another file`. The format is described in [Configuration](./configuration#export-file).

```sh
tunneler export > all-tunnels.toml
tunneler export --tag dev --tag web -o dev-web.toml
tunneler export --host nova --host vega -o two-hosts.toml --force
```

With `--json`, the output is `{"contents": "…", "tunnelCount": 3}`, where `contents` is the file as a string. With `-o` and `--json`, the file is written and the output is `{"path": "…", "tunnelCount": 3}` instead.

### `tunneler import <FILE>`

Adds the tunnels of a file made by `tunneler export`, or of a `tunnels.toml`, to your configuration. `FILE` is a path, or `-` for the standard input. The daemon is started when it is not running. The command never changes or removes a tunnel you already have: see [Move tunnels to another computer](../guide/export-import) for what it adds and what it skips.

It prints the plan first: for each tunnel of the file, its route, its tags, its status (will be added, already configured with the tags not applied, port already used, repeated in the file, invalid), its risks and its warnings, and whether it is selected. Then:

| Situation | What happens |
|---|---|
| `--dry-run` | The plan is printed and nothing is added. Exit code 0. |
| `--yes` | The tunnels the plan selects are added without asking. |
| Declined at the prompt | It prints `Nothing was imported.` and exits with code 0. |
| In a terminal | It asks `Import N tunnels? [y/N]` and adds the tunnels the plan selects. With `--include-risky` and risky tunnels, it asks `Import 3 tunnels, 1 of them risky? [y/N]` instead. The answer is read from the terminal, so `tunneler import -` works. |
| No terminal and no `--yes` | The plan is printed and nothing is added. Exit code 5. |

Tunnels that are risky (listening beyond this computer, a remote tunnel to a target that is not this computer or listening beyond the server's loopback, a tag that starts at launch) are marked `not selected` in the plan and left out. `--include-risky` selects them too: the plan then marks them `will import (risky)`, keeps their risks, and counts them in the summary (`2 to import (1 risky)`). Without it, edit the file, or use the window. Added tunnels start stopped.

| Option | Meaning |
|---|---|
| `--host-map <OLD=NEW>` | Use the alias `NEW` of this computer for the host `OLD` of the file. Repeat it for several hosts. |
| `--dry-run` | Print the plan and add nothing. |
| `-y`, `--yes` | Do not ask. |
| `--include-risky` | Also import the risky entries (they are marked in the plan). |
| `--json` | Print the result as one line of JSON. It never asks: without `--yes` it prints the plan and exits with code 5. |

The file is read up to 256 KiB and 1000 tunnels. A bigger input is refused without being read in full.

```sh
tunneler import dev-web.toml --dry-run
tunneler import dev-web.toml
tunneler import dev-web.toml --host-map nova=nova-home --yes
tunneler import dev-web.toml --include-risky
ssh vega cat .config/tunneler/tunnels.toml | tunneler import - --dry-run
```

With `--json`, the output is `{"plan": …, "selected": […]}` with `--dry-run` or without `--yes` (the latter exits with code 5), and `{"plan": …, "selected": […], "report": …}` with `--yes`. `selected` lists the `index` of the entries this run adds (the importable entries, risky ones too with `--include-risky`), so a script can see what `--yes` would do. The plan is `{"source", "entries", "unknownHosts"}`: `source` is `export` or `store`, `unknownHosts` lists the hosts of the file that no `Host` entry of your `ssh_config` names exactly and that have no mapping, and each entry has:

- `index`: its position in the file, from 0.
- `tunnel`: what would be created, after the host mapping.
- `fileHost`: the host as the file writes it.
- `effectiveListenAddress`: the address it would listen on.
- `status`: `{"kind": "importable"}`, `{"kind": "alreadyConfigured", "existingId", "ignoredTags"}`, `{"kind": "listenConflict", "holderId"}`, `{"kind": "duplicateInFile", "earlierIndex"}` or `{"kind": "invalid", "messages"}`.
- `risks`: a list of `{"kind": "nonLoopbackListen", "address"}` (also for a remote tunnel that listens on a non-loopback address of the server), `{"kind": "remoteNonLoopbackTarget", "targetHost"}` or `{"kind": "startsOnLaunch", "tags"}`.
- `warnings`: a list of `{"kind": "unknownHost", "host"}` or `{"kind": "openerDropped", "opener"}`.
- `selectedByDefault`: `true` when the status is `importable` and there is no risk. `--include-risky` does not change it: read `selected` for what the run picks.

The report is `{"added": [{"index", "id"}], "skipped": [{"index", "status"}]}`, where `status` is one of the statuses above, as found when the tunnels were added.

Exit codes of `export` and `import`:

| Code | When |
|---|---|
| 0 | Exported, imported, nothing to import, declined at the prompt, or `--dry-run`. |
| 1 | The export is larger than 256 KiB (`export`). The file is unreadable, too big, not valid UTF-8, holds control characters, or is refused as not a Tunneler file or from a newer version. Also: an existing output file without `--force`, a write error, or a daemon error. |
| 2 | Invalid usage, such as a `--host-map` that is not `OLD=NEW` or that names the same `OLD` twice. |
| 3 | The daemon is not running (`export`). |
| 5 | A confirmation is needed but there is no terminal and no `--yes`. |

### `tunneler desktop-entry <install|uninstall>`

Linux only. Adds Tunneler to the application menu, or removes it again. The archive has no installer, so this is how it gets a menu entry. See [Getting started](../guide/getting-started#linux).

`install` writes `com.minimalstuff.tunneler.desktop` and three icons under the user data directory (`$XDG_DATA_HOME`, usually `~/.local/share`), pointing at the `tunneler-ui` next to `tunneler`. Run it again after an update that moves the programs to another folder. It also points an existing launch-at-login entry at this `tunneler`, and never creates one. It refuses to run from `/usr` (but not `/usr/local`), where a package ships its own entry, and never overwrites a `.desktop` file that Tunneler did not write. It writes only the files that are missing or different, prints `Wrote` or `Unchanged` for each, and starts nothing. The entry also registers the `tunneler://` link scheme (`MimeType=x-scheme-handler/tunneler;`, and `%u` on the `Exec` line), see [Open an instance in one step](../guide/devslot#open-an-instance-in-one-step). After writing it, `install` runs `update-desktop-database` on the applications directory when that program exists, and ignores a failure.

`uninstall` removes the entry and the icons. It refuses to remove a `.desktop` file that Tunneler did not write.

On other systems both fail with exit code 2. A refusal (no `tunneler-ui` next to `tunneler`, a file in the way, an unwritable directory) exits with 1.

```sh
tunneler desktop-entry install
tunneler desktop-entry uninstall
```

## Output

- `--json` is on `status`, `list`, `devslot`, `export` and `import`. The output is one line, meant for scripts.
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
| 4 | Some tunnels failed to start or are blocked by another program (`up`), or some web ports of the instance failed, or were skipped because the local port is busy, a stale tunnel needs a decision or a tunnel you configured already forwards the port (`devslot`). A tunnel provided by an external `ssh` counts as started. |
| 5 | A confirmation is needed but cannot be asked: not a terminal, pass `--yes` (`quit`, `purge`, `import`). |
| 6 | Timed out waiting, or the command on the host timed out (`devslot`). |
| 7 | The target of the tunnel does not answer (`check`). |
| 8 | The tunnel did not become ready, or the browser or application could not be launched (`open`, `devslot`). |
| 9 | The application given with `--with` is not available (`open`). |

## Environment variables

| Variable | Meaning |
|---|---|
| `NO_COLOR` | Turn colour off. |
| `TUNNELER_LOG` | Log filter for the daemon and the commands, such as `debug`. |
| `TUNNELER_SOCKET`, `TUNNELER_CONFIG`, `TUNNELER_STATE`, `TUNNELER_SSH_CONFIG`, `TUNNELER_NO_TRAY` | Same as the matching option. |
| `TUNNELER_UI_BINARY` | The `tunneler-ui` program the daemon launches. |
