# Design decisions

These decisions are settled. Changing one means updating this page with the reason.

## Decisions

### Embedded SSH client
Tunneler uses the `russh` library and never runs the system `ssh`. One session per host carries every tunnel as a channel, so toggling a tunnel does not reconnect. It also behaves the same on every OS and gives full control over states, metrics, prompts and X11 without `xauth`.

### A daemon plus an on-demand UI
The daemon is headless (tray, SSH, IPC) and stays small. The Tauri window is a separate process that exists only while it is open, so a closed window costs no memory. The webview alone takes 100 to 200 MiB.

### Own `ssh_config` parser
The existing Rust crates lack `Match` and token expansion, or are too limited. The parser is written in-house and checked against `ssh -G`, so OpenSSH is the reference for behaviour.

### Own `known_hosts` parser
The one in `russh` has no wildcards or negation, no `@cert-authority` or `@revoked`, a fixed path, and writes entries in clear text. The in-house one handles all of that, including hashed entries.

### Tauri, React and `@minimalstuff/ui` for the UI
Tauri 2 with React 19, TypeScript, Vite and UnoCSS, using the author's own component library, which has a sound ARIA base. The webview memory cost is only paid while the window is open.

### Tray through `tray-icon`
Native Win32 on Windows. On Linux it uses the StatusNotifierItem protocol over D-Bus (`ksni`), so the daemon has no GTK dependency. There is no tray on macOS, because its event loop would need the main thread.

### Local IPC with JSON lines
`interprocess` gives a Unix socket or a Windows named pipe, restricted to the current user. The UI and the CLI speak the same protocol.

### Hand-editable TOML for the configuration
`tunnels.toml` lives in the user's config folder, or next to the executables in portable mode. It can be edited by hand, diffed, and is reloaded while the daemon runs. An installed copy never stores data in its install folder: updates replace it, uninstalling deletes it, and `Program Files` is not writable. A portable copy keeps everything in its own folder.

### Tags, not groups
A tunnel can belong to several sets (`dev`, `mail`, `adonis`). Toggling a tag toggles its tunnels.

### The daemon is the single instance
Binding the IPC socket is the lock. A second launch becomes a client that asks the daemon to show the UI, so no single-instance plugin is needed.

### Front-end tooling
oxlint and oxfmt through the shared `@minimalstuff/tooling` presets, plus oxlint's `jsx-a11y` plugin, which the React preset lacks. `rustfmt` and `clippy` for Rust. Fast, and the same as the author's other projects.

### Accessibility is a requirement
The UI targets WCAG 2.2 AA and the CLI must work without colour and without a terminal. A feature is not done until it works with the keyboard alone and with a screen reader, and does not rely on colour. See the [accessibility rules](/contributing/accessibility).

### Tags are the only naming
Tunnels have no name. A name repeated the address (`3333 → nova:3333` already identifies a tunnel) and was one more field to keep unique. A tunnel is shown by its route and selected on the command line by port, by `host:port` or by tag. For that reason a tag cannot be only digits or contain `:`: it would be mistaken for a port or a `host:port`.

### Standby: listen locally, open SSH on demand
A standby tunnel holds its local port and keeps incoming connections waiting. The first one opens the SSH session, then the waiting connections are attached to channels. No SSH session runs until someone connects. The daemon already owns the listener, so no extra process is needed. It is opt-in, with a general setting and a per-tunnel override. It applies to local and dynamic tunnels only. After `standby_idle_s` without a connection (5 minutes by default) the tunnel lets go of the session and goes back to standby.

When the session cannot open, the waiting connections are closed, a notification is sent, and the tunnel returns to standby with the reason. It never retries in a loop: the next connection tries again. The wake-up signal is a notification, a sound or both, and the daemon plays the sound itself so it stays audible when system notifications are off. The sounds are synthesised in the daemon: a system sound set to "none" still reports success and plays nothing, and the daemon cannot detect that.

### Opening a tunnel is the daemon's job
The daemon implements opening (browser or external application) once, for the UI, the CLI and the palette. Commands come from the user's own config, never from IPC parameters, and run without a shell.

### Distribution
A portable zip and tar.gz, an NSIS installer on Windows, a `.deb` on Linux, a `purge` command to clean up, and signed updates through GitHub releases. The Tauri bundler produces NSIS and `.deb` with no other tool. NSIS rather than MSI allows a per-user install without administrator rights, a native choice between user and machine, and install hooks. Cleaning lives in the binary, as one tested implementation that knows the paths and the keyring, and it serves the uninstaller, the portable copy and Linux. Only the Windows installer updates itself. The other forms are told that a new version exists.

## Closing the app

Tunnels live inside the daemon, so they cannot outlive it.

- **Closing the window** ends the UI only. The daemon and the tunnels keep running, with no confirmation.
- **Quitting Tunneler** from the tray or the UI, with active tunnels, asks for confirmation: stop everything and quit, or cancel.

## Decided against

### Leaving tunnels running after quitting
Tunnels live in the daemon, so quitting the daemon ends them. An option to quit and keep them running cannot exist with this architecture.

### A periodic health probe
Services that defend against brute force (Xvnc, fail2ban, sshd's `MaxStartups`) count every unauthenticated connection as a failure. A probe every few seconds ends with 127.0.0.1 banned. Link liveness comes from the SSH keepalive. A target is tested only when the user asks, with one connection that closes at once. A refused connection also marks the target unreachable, with no extra traffic.

### Standby for remote tunnels
A remote tunnel listens on the server, so holding the port needs a live session. That is the opposite of standby.

### Reconnecting from standby
When the session of an awake standby tunnel is lost, the tunnel goes straight back to standby. Its channels died with the session, so no connection is left to save, and an automatic reconnection would only reopen a session nobody uses.

### Standby for temporary tunnels
A temporary tunnel is an explicit, immediate request, so it starts active.

### A name on tunnels
See "Tags are the only naming" above. A `name` left in an old `tunnels.toml` is ignored on read and disappears on the next write.

### A shell to run openers
Applications are started from an argument list, with only `{host}`, `{port}` and `{url}` substituted. A shell would turn any value that ends up in a command into an injection risk.

### Opening a remote host name
Opening a tunnel uses `localhost`, the loopback address, or the exact IP the tunnel listens on, never a remote host name. A remote name would bypass the tunnel.

### A built-in SQL client preset
Arguments differ for every database driver, so there is nothing sensible to preset. Add your own application with the ports it should be suggested for.

### Single-click bypass of a changed host key
A changed or revoked host key is a hard failure with an explicit alert. There is no "connect anyway" button.

### Updating the AppImage on Linux
The Tauri updater can only update the AppImage, which does not contain the daemon. The UI and the daemon would end up on different versions. Portable, `.deb`, tar.gz and AppImage only show a banner with a link.

### The tray as the only way to do something
Everything the tray does can also be done from the UI and the command line.
