# Tunnels started outside Tunneler

You may already have `ssh -L 3333:...` running in a terminal or a script. Tunneler notices such tunnels so that it does not fight them for the port.

## How they are detected

Tunneler looks at who listens on the local port of each of your tunnels:

- when you start a tunnel;
- when the window opens;
- every few seconds while the window is open.

It also lists `ssh` processes that forward ports and that match none of your tunnels. Without an open window there is no periodic scan: `tunneler status` shows the result of the last one.

## A port held by an `ssh` that makes the same forward

When the process on the port is an `ssh` whose `-L`, `-R` or `-D` matches the tunnel (same kind, same port, compatible address, same target), the tunnel shows the state "External" with "provided by" and the process. Tunneler considers it already running and does not start a second one. The destination host of that `ssh` is not compared.

Three actions are offered:

- **Adopt.** Stops that process and starts every tunnel it provided as a Tunneler tunnel. Use it to move a forward under Tunneler's care.
- **Stop the external process…** Stops the `ssh` process, after a confirmation. Every tunnel that process provides stops too.
- **Ignore.** Stops reporting that process until it exits. This is not saved.

`tunneler up` counts a tunnel provided by an external `ssh` as started.

## A port held by something else

When another program holds the port, the tunnel shows "Conflict" and "port in use by" the program and its pid, for example `node.exe (pid 4242)`. The tunnel does not start. "Stop the process…" stops that program after a confirmation, and the tunnel can be started afterwards.

On Linux, the sockets of another user show no pid unless you are root, so the owner reads "unknown process".

## Tunnels Tunneler does not know

`ssh` forwards that match no configured tunnel appear in the side panel under "Unmanaged externals". For each one:

- **Import as a tunnel** opens the new tunnel form filled in from the process. Save it and it becomes a normal tunnel.
- **Stop the process…** and **Ignore** work as above.

A process is identified by its pid and its start time, so Tunneler never stops another program that reused the pid. Tunneler leaves its own process out of the scan.

There is no `adopt`, `kill` or `ignore` on the command line.
