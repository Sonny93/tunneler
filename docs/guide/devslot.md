# Tunnels from devslot

devslot gives each checkout of a project its own ports and compose project on a machine. When your instances run on a host you reach over SSH, Tunneler can list them and open the tunnels to their ports in a few clicks, instead of you typing each port.

## Discover the instances

1. Open the import panel and choose the "From devslot" tab.
2. Pick the host, an alias of your `ssh_config`, and select "Discover".
3. Each instance is listed with its ports. Web ports are checked, except those below 1024. Service ports (a database, for example) are not. A port that already has a tunnel is disabled and says so.
4. Select "Start N tunnels".

The ports an instance forces in its own configuration can be shared with other instances: they are marked as forced, or as ambiguous when several instances list the same one, so you can tell which one you are reaching.

Tunneler runs `ds ls --json` on the host, over the SSH session it already shares with that host's tunnels. This is the only command it ever runs there, and only when you select "Discover". A discovery that takes more than 20 seconds is cancelled, apart from the time spent waiting for a passphrase or password.

Every discovered tunnel is a local tunnel that listens on `127.0.0.1`, whatever the default bind address in the settings, and reaches the same port on the host. If that port is taken here, change the local port in the list. Links the app builds (such as an absolute URL in a page) still point at the remote port, so they stop working when the two ports differ.

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
