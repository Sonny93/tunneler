# Tunneler

SSH tunnel manager for Windows and Linux: a light daemon with an embedded SSH client and a tray icon, plus a window you open when you need it.

Status: early development.

## Why

Keeping SSH port forwards up by hand (one terminal per `ssh -L`, restarting them after sleep or a dropped network) is tedious and fails silently. Tunneler reads the user's `~/.ssh/config` and `known_hosts`, keeps the forwards running and reconnects them, and opens each one in the right program (browser, VNC or RDP viewer).

- **Users**: developers and administrators who reach services behind SSH from a desktop, on Windows and Linux. The author is the main user.
- **Stakes**: Tunneler holds the user's tunnel list (`tunnels.toml`), its state (`state.toml`, which lists the passphrases kept in the system keyring) and trusts host keys on the user's behalf. Losing or misreading those files, accepting a changed host key, or leaking a passphrase are the failures that matter.
- **Hard constraints**: no system `ssh` needed; the daemon stays small and the window costs nothing while closed; files stay hand-editable; nothing contacts the network on its own except the update check; accessibility is part of done. The reasons are in [Design decisions](https://sonny93.github.io/tunneler/roadmap/decisions).
- **Non-goals**: macOS (not maintained), being a general SSH client or terminal.

## Install

Download from the [releases page](https://github.com/Sonny93/tunneler/releases). The binaries are not signed.

- Windows: installer (`-setup.exe`) or portable zip. [Details](https://sonny93.github.io/tunneler/guide/getting-started#windows)
- Debian and Ubuntu: `sudo apt install ./tunneler_<version>_amd64.deb`
- Other Linux: `tar.gz` archive or AppImage. [Details](https://sonny93.github.io/tunneler/guide/getting-started#linux)

## Documentation

<https://sonny93.github.io/tunneler/>

## Development

```sh
cargo test --locked          # Rust crates
cd app && pnpm check         # front end
cd docs && pnpm build        # documentation site
```

See [Development](https://sonny93.github.io/tunneler/contributing/development) for the rest.
