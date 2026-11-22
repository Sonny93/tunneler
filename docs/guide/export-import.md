# Move tunnels to another computer

You set up your tunnels on one computer and want them on another. Export them to a file, copy the file, and import it there. The import only adds tunnels: it never changes or removes one you already have.

## Export

In the window, open the "Import and export" drawer, then the "File" tab. The "Export to a file" section has two optional filters, "Only these hosts" and "Only these tags". A tunnel is exported when it matches any of the filters, and without a filter every saved tunnel is exported. "Export…" opens the system dialog that asks where to save the file. Cancelling it does nothing.

From a terminal:

```sh
tunneler export -o tunnels-export.toml
tunneler export --tag dev --host nova > dev-and-nova.toml
```

Without `-o`, the file goes to the standard output. `-o` refuses an existing file unless you add `--force`. See [Command line](../reference/cli#tunneler-export).

### What the file contains

For each saved tunnel: its host, kind, ports and addresses, tags, [open options](./applications) and standby choice. The tunnels come in the order of your configuration. The format is described in [Configuration](../reference/configuration#export-file).

### What it does not contain

- Tunnel ids: the other computer gives its tunnels new ones.
- Settings, applications you added, X11 launchers and per-host settings.
- Passwords and passphrases. They are never in your configuration, so there is nothing to leak, and you are asked for them again on the other computer.
- Temporary tunnels.

The hosts are aliases of your `ssh_config`. The file does not carry that configuration: the other computer needs the same aliases, or you map them at import time (see [Hosts that this computer does not know](#hosts-that-this-computer-does-not-know)).

## Import

In the "File" tab, "Choose a file…" opens the system dialog and shows a preview of the file. Nothing is added until you confirm with "Import N tunnels". From a terminal:

```sh
tunneler import tunnels-export.toml
```

`tunneler import` prints the preview and asks for confirmation, so you can read it first. See [Command line](../reference/cli#tunneler-import-file) for the options, including `--dry-run` to only look.

The file can be one made by `tunneler export`, or a `tunnels.toml` copied as is. In the second case only the `[[tunnels]]` are read: settings, applications, X11 launchers and host settings are ignored, and the ids are replaced by new ones.

Imported tunnels start stopped.

### What the preview tells you

Each tunnel of the file is shown with its route, its tags and what will happen to it:

| Status | Meaning |
|---|---|
| Will be added | Nothing in your configuration stands in the way. |
| Already configured | You already have a tunnel with the same host, kind, addresses, ports and target. It is skipped. When the file gives it tags that yours lacks, they are listed as not applied: an import never edits a tunnel. |
| Port already used | Another tunnel of yours already listens on that address and port (or, for a remote tunnel, on that port of that host). It is skipped, and the preview names the tunnel. Tunneler does not pick another port for it. |
| Same route as an earlier tunnel of the file | The file lists it twice. Only the first one can be added. |
| Invalid | The entry breaks a rule of the configuration, such as a host name that is refused (see [Host names](../reference/configuration#host-names)). The preview gives the reason. |

If your configuration changes between the preview and the confirmation, Tunneler checks again before it writes anything. A tunnel that is no longer importable is skipped and reported.

### Hosts that this computer does not know

When no `Host` entry of your `ssh_config` names a host of the file exactly, the preview warns about it. A wildcard block such as `Host *.example` may still match, so it is a warning, not an error: Tunneler does not evaluate your `ssh_config` to find out.

To use another alias of this computer instead, pick it in the "Use this alias for nova" choice shown for the host `nova` (or "Keep nova" to leave it). The preview runs again with the choice. From a terminal, `--host-map OLD=NEW` does the same.

### Applications missing on this computer

When a tunnel opens with an application that this computer does not have (neither the browser, nor one you added, nor one that was detected), the import drops that choice and says so. The tunnel is added and opens with the usual application for its port.

### Tunnels that start unchecked

Some tunnels are risky to add without looking. They can be imported, but they are not checked by default, and the preview says why in text:

- **It listens beyond this computer.** A local or dynamic tunnel whose listen address is not a loopback address can be reached by other machines on your network.
- **It sends remote connections somewhere else.** A remote tunnel whose target is not this computer lets the server reach another machine from yours.
- **It listens beyond the server's loopback.** A remote tunnel that listens on a non-loopback address of the server, such as `0.0.0.0`, may be reachable by other machines there when the server's `GatewayPorts` allows it.
- **Its tags start at launch.** A tag of the tunnel is in "Tags to start on launch" here, so the tunnel would start by itself the next time Tunneler launches.

Check them yourself in the window. The command line imports the default selection only, so edit the file, or use the window, to import a tunnel that starts unchecked.

## A file from someone else

Treat a file you did not make as you would a script: read the preview. An imported tunnel can open a port on your computer, or make a server connect to a machine of the file's choosing. This is why the risky ones start unchecked, and why the import never touches what you have. Tunneler also refuses a whole file that contains control characters or invisible direction marks in a value, which could make a preview read differently from what it does, and it refuses host names that contain anything but letters, digits and `. _ - : @ [ ]` (see [Host names](../reference/configuration#host-names)).

## Limits

A file is at most 256 KiB and holds at most 1000 tunnels. A bigger one is refused before it is read. The export refuses to write more than 256 KiB, in the window and on the command line: filter by host or tag to export a part of your tunnels, then the others in another file. The preview can also refuse a file whose plan is too large to show, for example a very large file with very long host names.
