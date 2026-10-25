# Changelog

All notable changes to Tunneler are written here for the people who use it. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/) (0.x: a minor version may break compatibility).

## [Unreleased]

### Added

- The X11 drawer shows whether an X server answers on the local display, live, with a button to start VcXsrv or Xming when it is installed.

### Fixed

- A failed X server check is now reported instead of being silently ignored, and the daemon logs what it found with `TUNNELER_LOG=debug`. VcXsrv is also found in Program Files (x86).

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

[Unreleased]: https://github.com/Sonny93/tunneler/compare/v0.4.0...main
[0.4.0]: https://github.com/Sonny93/tunneler/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Sonny93/tunneler/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Sonny93/tunneler/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Sonny93/tunneler/releases/tag/v0.1.0
