# Roadmap

What Tunneler does today, what comes next, what needs testing on real machines, and what it does not do yet. The reasons behind the main choices are on the [Design decisions](./decisions) page.

## Shipped

### Tunnels
- Local (`-L`), remote (`-R`) and dynamic SOCKS5 (`-D`) forwards.
- One SSH session per host, shared by all its tunnels and closed shortly after the last one stops.
- Automatic reconnection with exponential backoff and jitter. A lost link puts the tunnels in a reconnecting state instead of failing them.
- Per-tunnel state, byte counters, active connections and a live log.
- A passive health mark: a refused connection to the target marks it unreachable. A one-off check on request (`tunneler check`, or "Check the target" in the UI).
- Temporary tunnels, started from the CLI with OpenSSH syntax (`tunneler up <host> -L 3333`), kept only while the daemon runs and savable into the config.
- Standby: a tunnel holds its local port and opens the SSH session on the first connection, then goes back to standby after a quiet period. Optional notification or sound when it wakes or fails to wake, with a volume setting for the synthesized sounds. Awake tunnels can be sent back to standby by hand, one at a time, by tag or all at once.
- X server check: before X11 or a launcher starts, Tunneler looks for an X server on the local display. When none runs, it can start an installed VcXsrv or Xming (ask, always or never), or warns that windows will not appear.
- X11 forwarding without the `xauth` binary, with an anchor per host and saved launchers.

### SSH
- Embedded client: no system `ssh` needed.
- A complete `ssh_config` parser, checked against `ssh -G`: `Host`, `Match`, `Include`, tokens, `ProxyJump`, `ProxyCommand`, keepalive options.
- Authentication in OpenSSH order: agent (Unix socket, OpenSSH named pipe or Pageant), key files (OpenSSH, PKCS#8, PuTTY), keyboard-interactive, password.
- Key passphrases can be remembered for the session or in the OS keyring. Passwords are never remembered.
- A `known_hosts` parser with wildcards, negation, hashed entries, `@revoked` and `StrictHostKeyChecking`. A changed host key is a hard failure.

### Tunnels started outside Tunneler
- Detection of `ssh` processes that hold a configured tunnel's port, and of `ssh` tunnels that are not configured at all.
- Adopt, kill, ignore or import them as a configured tunnel.
- Ports held by other programs are shown as conflicts, with the program and its pid.

### Interface
- Tray icon whose shape and tooltip show the global state, with a menu to toggle tags.
- On-demand window: the process exits when the window closes, the tunnels keep running in the daemon.
- Tunnel list grouped by host, with tags, multi-selection, context menus and a command palette (Ctrl+K).
- Detail drawer with live logs and metrics, an import from `ssh_config` and from pasted `ssh -L` scripts. The drawers slide in and out.
- A settings window with vertical tabs (General, Connection, Standby, Opening, Secrets), one "Save" for everything including your applications, a count of errors on each tab, a confirmation before discarding changes however you close the window (Cancel, Escape, a click outside or the close button), and one command per tab in the palette.
- Open a tunnel in the browser or in an application picked from the target port (VNC, RDP, any command you add). A detected application can be customized, for example to point at a portable copy. A setting makes tunnels without a saved application always open in the browser, and a double-click on a row opens the tunnel.
- This documentation, in English and French, opened from the app with F1.
- Dark and light themes that follow the system.
- English and French, for the window, the tray and the notifications. The language follows the system or is picked in the settings.

### Command line
- `tunneler up`, `down`, `status`, `list`, `open`, `check`, `quit` and `purge`, with `--json` output and documented exit codes.
- Tunnels are selected by port, by `host:port` or by tag.

### Distribution
- Windows installer (per user or per machine), Debian package, portable zip and tar.gz, and an AppImage of the UI.
- Portable mode: everything lives next to the executables.
- `tunneler purge` removes every trace the program left on the machine.
- Signed updates from GitHub releases, applied automatically for the Windows installer and offered as a link for the other forms. A check runs when the window opens, or on demand from the settings and the palette.
- Daemon memory: about 17 to 25 MiB. The UI only uses memory while its window is open.

## Next

- Whatever the [Needs testing](#needs-testing) section turns up.

## Needs testing

Most of the code runs in automated tests against an in-process SSH server. Real machines are what is missing. If you try any of these, an issue with the result, good or bad, helps.

### Windows
- **Installer.** Install for the current user and for all users, with and without the shortcuts. Update from the previous version (the tunnels should come back). Uninstall with and without the "remove my tunnels and settings" box: nothing should be left behind.
- **Everything the daemon does on Windows.** It compiles and passes lint but has not been exercised on every path: the named pipe, the tray, launch at login, the credential manager, the OpenSSH agent and Pageant, `ProxyCommand` and `Match exec`, detection of VNC and RDP clients, detached launch.
- **Notifications.** A toast should appear under the name Tunneler without a Start menu shortcut.
- **Standby sounds.** Listen to all of them, including with the "Notification" sound set to "(None)" in the Windows sound scheme. Check that none ends abruptly (each rings out, then 150 ms of silence) and that the volume slider is neither too quiet at 5 % nor too loud at 100 %.
- **X server start.** Starting VcXsrv and Xming when no X server runs, with each setting value.
- **Screen reader and display.** NVDA with the UI, Windows high contrast mode, and 200% zoom with no horizontal scrolling.

### Linux
- **Debian package.** Install, run, update and remove the `.deb`.
- **Sounds and notifications.** Standby sounds on a real desktop with PipeWire (`pw-play -`), and with PulseAudio only (`paplay`, never tried). D-Bus notifications.
- **Tray under KDE.**
- **Screen reader.** Orca, listened to by a person. The keyboard pass and the accessibility tree were checked.
- **Memory.** The UI weighs about 410 MiB PSS under Xvfb with software rendering. A real desktop with a GPU should be lower: a measurement would confirm it.

### Both
- **X11.** VcXsrv on Windows and Xorg or Xwayland on Linux, against a real `sshd`: a remote `xeyes` should show up without `xauth`.
- **SOCKS.** `curl --socks5` through a dynamic tunnel to a real `sshd`.
- **The UI as a person sees it.** Forced standby (menu, selection bar, right-click on "All tunnels", tray entry), the drawers, the settings window and its tabs, the command palette, the collapsed side panel, tag rename and delete, the opening form and the applications section, standby badges, tooltips, right-click and multi-selection.
- **Closing the settings window with unsaved changes.** Escape, a click outside and the close button should each ask "Discard your changes?". "Keep editing" should put the focus back on the field you were editing.
- **Passphrase prompt.** Pressing Enter should submit it. The test environment cannot check it.
- **Accessibility pass on the release binaries.** WCAG 2.2 AA, on both systems. Beyond the points above: focus ring visibility in both themes, the arrow keys and Enter in the palette, the passphrase prompt taking focus, field errors being read when the field gets focus, the import checkboxes and warnings, system colour scheme, touch target size, and the CLI with `NO_COLOR=1`, `TERM=dumb` and without a terminal. The [accessibility rules](/contributing/accessibility) say what is expected.

## Known limitations

### SSH
- `ssh_config`: named ports, `~user/` paths, file permission checks and `CanonicalizeHostname` are not supported. `localnetwork`, `version`, `sessiontype` and `command` never match in a `Match` line. `~` expands from `$HOME`, not from the passwd entry. The Windows behaviours (`%PROGRAMDATA%`, `cmd /C`, globs) are untested.
- Host certificates are refused, so `@cert-authority` entries cannot be used. Revocation of the key inside a certificate is not checked, only that of the CA.
- `AddKeysToAgent` is not implemented. User certificates are wired in without an integration test. Only the OpenSSH key format is tested end to end.
- A jump host is not shared between two targets: each target opens its own connection to it.

### Tunnels
- Standby works for local and dynamic tunnels only. A remote tunnel listens on the server, so it needs a session.
- There is no periodic health probe, so a dead target is only noticed when a connection to it fails or when you ask for a check.
- Reloading a config where a setting goes from absent to its explicit default restarts the tunnel.
- X11: a host with no tunnel has no group, so no X11 access in the UI. The display of launchd on macOS is not supported, and X11 anchors are not restored at launch.

### Tunnels started elsewhere
- Without an open UI there is no periodic scan: `tunneler status` shows the last one.
- The `ssh` destination is not compared when matching a tunnel.
- On Linux, sockets of another user show no pid without root.
- There is no `adopt`, `kill` or `ignore` on the command line.

### Interface and command line
- The import does not take `$env:DISPLAY` from a script into `x11_display`, and there is no import on the command line.
- Health times on the command line are in UTC.
- A `tunnels.toml` with CRLF line endings is not guaranteed to be rewritten identically outside the lines Tunneler changes.
- The borders of some controls from the component library are below 3:1 contrast (WCAG 1.4.11). This is accepted for now.
- WebKitGTK exposes the actions menu button as a combo box.
- The "English" and "Français" choices of the language setting are not marked with their language for screen readers: the component library cannot set `lang` on an option yet.
- Windows toasts carry the name Tunneler but no icon.
- Binaries and installers are not signed: Windows SmartScreen warns on first run.
- macOS is not maintained: no package, no CI, no tray.
