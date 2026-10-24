# Changelog

All notable changes to Tunneler are written here for the people who use it. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/) (0.x: a minor version may break compatibility).

## [Unreleased]

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

[Unreleased]: https://github.com/Sonny93/tunneler/compare/v0.1.0...main
[0.1.0]: https://github.com/Sonny93/tunneler/releases/tag/v0.1.0
