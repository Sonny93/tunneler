# Tunnels from devslot

[devslot](https://github.com/Sonny93/devslot) gives each checkout of a project its own ports and compose project on a machine. When your instances run on a host you reach over SSH, Tunneler can list them and open the tunnels to their ports in a few clicks, instead of you typing each port.

To install devslot, follow the ["Install" section of its README](https://github.com/Sonny93/devslot#install): a prebuilt `.deb`, an Arch package or a tarball.

## Discover the instances

1. Open the "Import and export" drawer and choose the "From devslot" tab.
2. Pick the host, an alias of your `ssh_config`, and select "Discover".
3. Each instance is listed with its ports. Web ports are checked, except those below 1024. Service ports (a database, for example) are not. A port that already has a tunnel is disabled and says so.
4. Select "Start N tunnels".

The ports an instance forces in its own configuration can be shared with other instances: they are marked as forced, or as ambiguous when several instances list the same one, so you can tell which one you are reaching.

Tunneler runs `ds ls --json` on the host, over the SSH session it already shares with that host's tunnels. This is the only command it ever runs there, and only when you select "Discover". A discovery that takes more than 20 seconds is cancelled, apart from the time spent waiting for a passphrase or password.

Every discovered tunnel is a local tunnel that listens on `127.0.0.1`, whatever the default bind address in the settings, and reaches the same port on the host. If that port is already in use on your computer (for example by devslot running there too, since it hands out the same ports), or is the listening port of another tunnel of Tunneler, even a stopped one, Tunneler suggests a free local port (the remote port plus 10000, or the next free one) and fills it in; you can change it in the list. Links the app builds (such as an absolute URL in a page) still point at the remote port, so they stop working when the two ports differ.

## Open an instance in one step

When you already know which instance you want, skip the panel. One command, or one link, does the whole path: it finds the instance on the host, tunnels its web ports and opens the application in your browser.

```sh
tunneler devslot vega web-shop-098a
```

`vega` is the alias of the host in your `ssh_config`, as this computer knows it. `web-shop-098a` is the name of the instance as `ds ls` prints it: a main checkout or a worktree. The link form is `tunneler://devslot/vega/web-shop-098a`. Both segments are letters, digits, `.`, `_` and `-`, start with a letter or a digit, and are at most 64 (host) and 255 (instance) characters. The command and its options are in the [command line reference](../reference/cli#tunneler-devslot-host-instance).

When the server's devslot configuration has a `[tunneler] host = "<alias>"` entry, `ds info` prints this command and this link next to the preview URL. The server only prints text: nothing happens on your computer until you run the command or click the link.

### What it does

1. Runs `ds ls --json` on the host, like the discovery above.
2. Removes the stale devslot tunnels that need no question: the temporary ones, and the saved ones when `devslot_stale = "delete"`, under the same safety rule as above. The routes it removed are listed. A saved tunnel that would need your decision is left alone: the command says how many wait, and the "Import and export" drawer is where you answer.
3. Starts a tunnel for each web port of the instance that has none, as in the "Import and export" drawer. A port that already has a devslot tunnel of this host, listening on the loopback (`127.0.0.1`, `::1` or `localhost`, or the default bind address of the settings when the tunnel sets none), is reused, and started if it was stopped. Any other tunnel that forwards the port, one you configured yourself or one that listens on `0.0.0.0`, is yours: the port is skipped and you start that tunnel yourself. A tunnel listens on `127.0.0.1`, and takes a free local port when the remote one is in use on this computer.
4. Opens the application in the browser, at `http://<label>.localhost:<port>`, unless you pass `--no-open`.

Only web ports are tunneled. Services such as a database are not, and neither are ports below 1024 (left to the "Import and export" drawer). A port below 1024 is reported as skipped, by design. A port that is skipped because it is in use and no free port was found, because a stale tunnel holds it and waits for your decision, or because a tunnel you configured already forwards it, is reported too and makes the command exit with 4, like a port whose tunnel failed.

Running it twice creates nothing new: the second run finds the tunnels of the first, and only opens the browser again. When another instance lists the same port as the application, the browser may reach that other one, and the output says so.

The tunnels are temporary or saved following `devslot_persist`, like in the "Import and export" drawer (see [Temporary or saved](#temporary-or-saved)). Without a setting they are temporary.

### The link

Clicking a `tunneler://devslot/<host>/<instance>` link opens Tunneler, which always shows a confirmation naming the host and the instance before it makes any SSH connection. Any web page can point to a link of this kind, so the confirmation is what keeps a page from opening connections from your computer. A link that does not follow the format, or whose host is not an alias of your `ssh_config`, shows an error and nothing else happens. "Open" only responds one second after the confirmation appears or the window comes back to the front, so a click meant for the page that sent the link cannot land on it. While a confirmation is shown, a new link is ignored: answer the first one.

A browser usually asks before it hands a link to another application. A terminal may not make `tunneler://` links clickable: the command is the path that always works.

The link works when the program registered the `tunneler` scheme with your system:

- the Windows installer and the Debian package do it for you;
- for the Linux archive (`tar.gz`), run `tunneler desktop-entry install`, which adds the scheme to the menu entry it writes;
- the Windows portable zip, the AppImage and macOS do not register it: use the command there.

The alias comes from the server's configuration. A second computer that knows the same server under another alias gets links that fail with an unknown alias: type the command with the alias that computer uses.

## Where `ds` is

A command run by SSH without a terminal often has a shorter `PATH` than your shell: `~/.cargo/bin` or the shims of mise may be missing, and `ds` is then "not found". Set the full path under the host, in `tunnels.toml`:

```toml
[hosts.nova]
devslot_path = "~/.cargo/bin/ds"
```

The path is used as written, quoted for the remote shell. A leading `~/` is the exception: the remote shell expands it. Without `devslot_path`, Tunneler runs `ds`. See [Configuration](../reference/configuration#hosts-alias).

## Temporary or saved

By default the tunnels are temporary: they are not written to `tunnels.toml` and disappear when you stop them or when the daemon quits. Turn on "Keep these tunnels" to save them instead.

The switch starts from the `devslot_persist` setting, which a host can override with its own `devslot_persist`. Both are described in the [configuration reference](../reference/configuration).

A saved tunnel gets two tags: `devslot`, and `ds-<project>` for the project of the instance. Every character of the project name outside letters, digits, `.`, `_` and `-` becomes `-`, and the tag is cut at 64 bytes. An instance without a project name, or whose name leaves nothing once cleaned, gets only `devslot`.

A temporary tunnel carries no tag. It shows its project next to the "Temporary" badge, and Tunneler recognises it as a devslot tunnel for as long as the daemon runs. When you save one ("Save tunnel" in its menu), it gets the same two tags.

## Open a web port

A web port of an instance has a name of the form `<label>.localhost`. The tunnel opens `http://<label>.localhost:<port>` in the browser, which resolves `.localhost` to your own computer, so each instance keeps its own cookies. Applications and other programs are not affected: they still connect to the usual loopback address. See `open.host_name` in the [configuration reference](../reference/configuration#open).

The URL uses the local port. When it differs from the remote one, the links the app builds point at the remote port.

## Keeping the tunnels in step

Each discovery compares the instances with the devslot tunnels of that host: the saved tunnels tagged `devslot` and the temporary ones from a discovery. Only local tunnels that reach `127.0.0.1` of the host count. A tunnel is stale when no instance lists its remote port any more, or when the port now belongs to another project or another host name.

- **Temporary tunnels** that are stale are removed at once, and the result is announced.
- **Saved tunnels** that are stale are listed with "Delete" and "Keep". "Keep" hides it for this discovery. With `devslot_stale = "delete"`, a saved tunnel whose port is gone is deleted without asking. A saved tunnel whose port changed project is always asked about.
- **Safety rule**: when the host lists no instance, or when every saved devslot tunnel of the host would be deleted, all of them are asked about instead. An empty listing then never wipes your tunnels.
- **Forced ports** that several instances share are never touched.

Only tunnels that match what the discovery returned are removed: a tunnel you edited in the meantime is left alone.

## When it fails

| Message | Cause |
|---|---|
| `ds` not found | The shell has no `ds` in its `PATH` (exit status 127): set `devslot_path`. |
| The command failed | `ds` exited with an error. The end of its error output is shown. |
| Invalid output | `ds` answered something Tunneler cannot read, or more than 1 MiB. |
| Timeout | No answer within 20 seconds. |

Connection, host key and authentication errors are the same as for any tunnel on that host.
