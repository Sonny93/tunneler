# Changelog

All notable changes to Tunneler are written here for the people who use it. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/) (0.x: a minor version may break compatibility).

## [Unreleased]

### Added

- devslot discovery: the import panel has a "From devslot" tab that lists the instances of a host (`ds ls --json` over SSH) and starts tunnels to their ports, temporary by default or saved with "Keep these tunnels" (tags `devslot` and `ds-<project>`). Each discovery removes the temporary tunnels whose port is gone and asks about the saved ones, or deletes them with `devslot_stale = "delete"`. Web ports open at `http://<label>.localhost:<port>`.
- `tunneler devslot <host> <instance>` opens a devslot instance in one step: it tunnels the web ports of the instance and opens the application in the browser. `--no-open` leaves the browser alone and `--json` prints the result for scripts. Running it again creates nothing new.
- The `tunneler://devslot/<host>/<instance>` link does the same from a page or a message. Tunneler always asks for confirmation, naming the host and the instance, before it connects. The Windows installer and the Debian package register the link, and so does `tunneler desktop-entry install` for the archive. The Windows portable zip, the AppImage and macOS do not.
- Settings `devslot_persist` and `devslot_stale`, and per host `devslot_path` (where `ds` is on the host) and `devslot_persist`, in `tunnels.toml`.
- `open.host_name` on a tunnel: a `<label>.localhost` name used in the URL opened in the browser.

### Changed

- The X11 drawer shows its sections as cards, like the tunnel details drawer: anchor, X server, remote display and launchers.
- The tray icon is now the Tunneler logo, its arch coloured by the global state (grey when idle, green when healthy, amber when degraded, red on error), instead of a different shape per state.
- The devslot import tab now suggests a free local port when the port of an instance is already the listening port of another tunnel, even a stopped one, as it already did for a port used by another program.
- `tunneler desktop-entry install` registers the `tunneler://` link scheme in the menu entry it writes, and asks the desktop to refresh its database when `update-desktop-database` is available. An existing entry gets the scheme the next time the daemon starts.
- Notifications now show above dialogs and drawers, and screen readers announce them even while one is open. F8 jumps to the notifications and back (it is listed in the keyboard shortcuts help), and Escape closes the focused one. Warnings stay until you close them, like errors.
- Dialogs, menus and form fields pick up the accessibility fixes of the UI library: loading buttons announce that they are loading, focus returns to the control that opened a dialog, and menus inside dialogs are reachable by screen readers.

### Fixed

- When the SSH connection fails while launching an X11 app or discovering devslot instances, the window now says which host could not be reached and why, instead of showing an internal error.

## [0.6.1] - 2026-11-18

### Fixed

- `tunneler desktop-entry install` no longer says it wrote the files it left as they were: it prints `Unchanged` for them.
- `tunneler purge --dry-run` no longer lists the launch-at-login entry when there is none.

## [0.6.0] - 2026-11-18

### Added

- `tunneler status` prints the path of the configuration file in use. When that file does not exist but earlier portable copies do, it lists them, and when a portable copy ignores the tunnels file in your user directory, it says so.
- `tunneler desktop-entry install` adds Tunneler and its icon to the application menu on Linux, for the tar.gz archive that has no installer; `uninstall` and `tunneler purge` remove them. The daemon keeps an existing entry pointing at the current program, and the getting started page shows a mise `postinstall` line that does the same at every install or upgrade.

### Changed

- The Linux archive (`tar.gz`) is no longer portable: it keeps your files in your user directories, so a new version no longer starts empty and mise no longer deletes your tunnels with the old version. Every earlier `tar.gz` was portable. Run `touch portable` next to the programs to keep the old behaviour.
- When you upgrade from an earlier `tar.gz`, the daemon copies `tunnels.toml` and `state.toml` from the earlier copy to your user directories at start, if that copy sits next to the new one, it is the only one, it is a real folder that you own and that others cannot write to (inside a folder others cannot write to either, so not under `/tmp`), and no tunnels file exists in your user directory yet. Nothing is moved, deleted or overwritten. Otherwise, run `tunneler quit`, copy `tunnels.toml` to `~/.config/tunneler/` and `data/state.toml` to `~/.local/state/tunneler/`. With mise, do it before the old version is pruned, or upgrade with `mise upgrade --no-prune`.
- An application installed while Tunneler runs (TigerVNC, Remmina, FreeRDP...) is now found without restarting Tunneler: the built-in applications are detected again when the window opens and before a tunnel is opened.

## [0.5.0] - 2026-10-29

### Fixed

- Picking an option in a settings card, such as "When no X server is running", no longer scrolls the settings window and leaves half of it blank. Closing a menu or a dialog no longer scrolls the window behind it either.

## [0.4.1] - 2026-10-25

### Added

- The X11 drawer shows whether an X server answers on the local display, live, with a button to start VcXsrv or Xming when it is installed.

### Fixed

- A failed X server check is now reported instead of being silently ignored, and the daemon logs what it found with `TUNNELER_LOG=debug`. VcXsrv is also found in Program Files (x86).
- VcXsrv and Xming started by Tunneler refused the windows of X clients: they now start from their install folder, like XLaunch does, so they find the `X0.hosts` file that lets local clients in.

## [0.4.0] - 2026-10-24

### Added

- A volume setting for the standby sounds (Settings › Standby): a slider from 5 to 100 %, 50 % by default, for the chime, marimba and plucked string. The preview plays at the slider's value before you save. The system sound keeps following the system volume. `standby_volume` in `[settings]`.
- Tunneler checks for a running X server before starting X11 or a launcher, and can start VcXsrv or Xming when one is installed. Settings › Connection › "When no X server is running" chooses between asking, starting it and not starting it; `x11_server_start` in `[settings]`.

### Changed

- The window, the daemon and the `tunneler` command speak a new version of their protocol, and refuse to talk to a 0.3.0 counterpart.
- The tunnel form groups its fields into cards: General, Forwarding, Opening and Standby.

### Fixed

- The window no longer shows a second scrollbar next to the tunnel list's.
- Tooltips no longer stay open after a click: they show on hover and on keyboard focus, never on touch. The chevron next to "New tunnel" gets its tooltip back.
- The standby sounds are no longer cut before their end: each note rings until it has faded, followed by a short silence so the player does not clip it.

## [0.3.0] - 2026-10-24

### Added

- Double-clicking a started local tunnel in the list opens it with its default action, like "Open" in its menu. A double-click on a stopped tunnel does nothing.
- A "Tunnels without an application" setting (Settings › Opening) makes tunnels with no saved application always open in the browser instead of the application suggested for their port (`open_default` in `[settings]`).
- A "Check now" button (Settings › General › Updates) and a "Check for updates" palette command look for a new version at any time, even with the automatic check turned off.

### Fixed

- The header and the tunnel list no longer scroll away together: the header stays in place, and the sidebar and the list scroll on their own.
- The chevron next to "New tunnel" now has the same height and colour as the button.

## [0.2.0] - 2026-10-24

### Added

- A settings window with vertical tabs (General, Connection, Standby, Opening, Secrets) replaces the settings drawer. One "Save" button saves everything at once, your applications included.
- Each settings tab shows how many of its fields have an error, and saving takes you to the first one, even in another tab.
- Closing the settings with unsaved changes asks first, whichever way you close them: Cancel, Escape, a click outside or the close button.
- One command per settings tab in the command palette ("Settings: General", "Settings: Connection"…).
- The X11 drawer shows the local display X11 forwards to: the host's `x11_display`, else the general setting, else the `DISPLAY` variable.

### Changed

- The tunnel details drawer shows its sections as cards, in the order state, actions, metrics and log. Metrics are tiles, and the log fills the remaining height.
- The details drawer is wider, and every drawer slides in and out.
- Single-group settings tabs (Standby, Secrets) no longer repeat their name as a group title.
- The window, the daemon and the `tunneler` command speak a new version of their protocol, and refuse to talk to a 0.1.0 counterpart.

### Fixed

- Drawers took the whole window width on wide screens.

## [0.1.0] - 2026-10-22

First release.

### Added

- Local (`-L`), remote (`-R`) and dynamic SOCKS5 (`-D`) tunnels, with one SSH session per host shared by its tunnels, automatic reconnection, a live log, byte counters and active connections.
- Standby: a tunnel holds its port and connects on first use, then goes back to standby when idle, with an optional notification or sound.
- Temporary tunnels started from the command line with OpenSSH syntax.
- X11 forwarding without `xauth`, with an anchor per host and saved launchers.
- An embedded SSH client that reads your `ssh_config` (`Include`, `Match`, `ProxyJump`, `ProxyCommand`…) and `known_hosts`, and authenticates with the agent, key files or a password. Key passphrases can be kept in the system keyring.
- Detection of `ssh` tunnels started outside Tunneler, which can be adopted, stopped, ignored or imported.
- A tray icon showing the global state, and a window opened on demand with the tunnels grouped by host, tags, multi-selection, a command palette (Ctrl+K), imports from `ssh_config` and from pasted `ssh -L` commands, and opening a tunnel in the browser or an application picked from its port.
- The `tunneler` command line: `up`, `down`, `status`, `list`, `open`, `check`, `quit` and `purge`, with `--json` output.
- English and French, dark and light themes, and documentation opened from the app with F1.
- A Windows installer, a Debian package, an AppImage and portable archives, with signed updates from the GitHub releases.

[Unreleased]: https://github.com/Sonny93/tunneler/compare/v0.6.1...main
[0.6.1]: https://github.com/Sonny93/tunneler/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/Sonny93/tunneler/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Sonny93/tunneler/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/Sonny93/tunneler/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Sonny93/tunneler/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Sonny93/tunneler/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Sonny93/tunneler/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Sonny93/tunneler/releases/tag/v0.1.0
