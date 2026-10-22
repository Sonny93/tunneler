# Tunnels

A tunnel is one port forward through one host of your `ssh_config`. Tunnels have no name: the list shows where each one listens and what it reaches, such as `3333 → nova:3333`. Tunnels that go through the same host share one SSH session.

## Kinds

### Local, `-L`

Listens on your computer and connects to the target from the server. Use it to reach a service that only the server can see: a database, an admin page, a VNC desktop.

`3333 → nova:3333` listens on `127.0.0.1:3333` here and reaches port 3333 of `nova`. To reach another machine through `nova`, set "Target host" (for example `db.internal`) and "Target port". The list then shows `3333 → db.internal:5432 via nova`.

```sh
tunneler up nova -L 5000:db.internal:5432
```

### Remote, `-R`

Listens on the server and connects to the target from your computer. Use it to let the server, or someone on it, reach a service running on your machine.

`nova:9000 → 127.0.0.1:3000` listens on port 9000 of `nova` and reaches port 3000 here. By default Tunneler asks the server to listen on `localhost`; set "Listen address" to change it. The server's own `GatewayPorts` setting has the final word.

```sh
tunneler up nova -R 9000:localhost:3000
```

### Dynamic, `-D`

Listens on your computer as a SOCKS5 proxy. Each client says where it wants to go and the connection leaves from the server. Use it to browse or run a tool as if you were on the server's network. A dynamic tunnel has no target.

```sh
tunneler up nova -D 1080
curl --socks5-hostname 127.0.0.1:1080 http://intranet.example/
```

## Tags

A tag groups tunnels. Give a tunnel one or several tags, separated by commas, in its form. A tag has no spaces, no comma, no colon, and is not made of digits only: the command line would read a number as a port and `host:port` as a pair.

The side panel lists the tags with how many tunnels are active. Use a tag's switch to start or stop the whole group. From a terminal:

```sh
tunneler up dev
tunneler down dev
```

To start a tag when Tunneler launches, add it to "Tags to start on launch" in the settings. Use a tag's menu in the side panel to rename or delete it. Deleting a tag never deletes its tunnels.

## Standby

A tunnel in standby holds its local port but opens no SSH session. The session opens when something connects to the port, and the connection waits until the tunnel is ready. When no connection has been open for a while, the tunnel goes back to standby and the session closes.

It fits tunnels you keep configured but use now and then: no idle connection to the server, no keepalive traffic, and a prompt for your passphrase only when you need it. If the connection fails, the waiting connections are closed, you get a notification, and the tunnel goes back to standby. The next connection tries again.

- Standby works for local and dynamic tunnels. A remote tunnel listens on the server, so it needs a session.
- Turn it on for every tunnel with "Put tunnels on standby" in the settings, or per tunnel with the "Standby" choice in the tunnel's form: "General setting", "Yes" or "No". A change applies the next time the tunnel starts.
- "Return to standby after (seconds)" sets the quiet time. The default is 300.
- "Signal when leaving standby" chooses what happens when a tunnel wakes up or fails to: nothing, a silent notification, a sound, or both. The sound can be a chime, a marimba, a plucked string, or the system sound, and each has a preview button.
- A tunnel that woke up can be sent back to standby by hand, one at a time, by tag, or all at once: "Send to standby" in the menus, or "Send awake tunnels back to standby" in the tray menu.
- Temporary tunnels never wait: they are an explicit request and start active.

## Temporary tunnels

A temporary tunnel is started from the command line with the `ssh` syntax. It is never saved to `tunnels.toml`, and it disappears when you stop it or when the daemon quits.

```sh
tunneler up nova -L 5000
tunneler up nova -D 1080
tunneler down 5000
```

In the window it shows a "Temporary" badge. To keep one, use "Save tunnel" in its menu. You can also create one from the window with "Temporary tunnel…" in the menu next to "New tunnel".

If a saved tunnel already makes the same forward, `tunneler up` starts that one instead.

## Health

A tunnel's state says whether the link to the server works. It does not say whether the service behind it answers. Tunneler learns that passively: when the server refuses a connection to the target, the tunnel is marked "Target unreachable" with the reason.

Tunneler never probes the target on a timer. Some services, such as Xvnc, fail2ban or the `MaxStartups` of `sshd`, count each connection that closes without logging in as a failed attempt, and a probe would end up getting your address blocked.

To test once, on demand, use "Check the target" in the tunnel's details, or:

```sh
tunneler check 3333
```

The check opens one connection to the target and closes it at once. It exits with code 7 when the target does not answer. It works on a running local or remote tunnel, not a dynamic one or one in standby.

## Closing the window and quitting

Closing the window closes the program that draws it. The daemon and the tunnels keep running.

To stop, choose "Quit" in the tray menu, "Quit Tunneler" in the window, or run `tunneler quit`. When tunnels are active it asks first, and offers "Stop everything and quit". The tunnels run inside the daemon, so they stop with it. Unless you turned off "Restore active tunnels on launch", they start again at the next launch.
