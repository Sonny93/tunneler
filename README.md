# Tunneler

Portable SSH tunnel manager (Windows + Linux; macOS is not maintained): a light headless daemon with an embedded SSH client and a tray icon, plus an on-demand Tauri UI.

> Status: early development. See [`docs/PLAN.md`](docs/PLAN.md) for the design and [`docs/TASKS.md`](docs/TASKS.md) for progress.

## Install

Packages are published as GitHub releases (and as artifacts of the `release` workflow runs). The binaries are not signed.

### Windows

Download `tunneler-<version>-windows-x86_64.zip`, extract it anywhere and run `tunneler-ui.exe` (or `tunneler.exe` for the command line). Windows SmartScreen warns about the unsigned binaries: choose "More info", then "Run anyway".

Keep `tunneler.exe` and `tunneler-ui.exe` side by side: the daemon looks for `tunneler-ui` next to itself, then on `PATH`, or at the path in `TUNNELER_UI_BINARY`.

### Linux

Download `tunneler-<version>-linux-x86_64.tar.gz`, extract it and run `./tunneler-ui` (or `./tunneler` for the command line). The binaries are built on Ubuntu 22.04 (glibc 2.35 or newer). The daemon only needs libc; the UI needs WebKitGTK 4.1 and GTK 3. On Debian and Ubuntu:

```sh
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0
```

These packages pull in the rest of the UI's shared libraries (`libsoup-3.0-0`, `libjavascriptcoregtk-4.1-0`, `libgdk-pixbuf-2.0-0`, `libcairo2`, `libglib2.0-0`, `libdbus-1-3`...). The tray icon and the keyring talk D-Bus directly, so no appindicator library is needed.

When `libwebkit2gtk-4.1` cannot be installed, download `tunneler-ui-<version>-x86_64.AppImage` instead, which bundles its libraries. Put it next to `tunneler` under the name `tunneler-ui`, or point `TUNNELER_UI_BINARY` at it.

### macOS

Not maintained: no package and no CI. The code may still build from source (see [Development](#development)).

### Memory use

| Platform | Daemon idle | Daemon, 10 tunnels | UI open |
|---|---|---|---|
| Linux | 16.5 MiB | 19.4 MiB | ~410 MiB |
| Windows | 25.2 MiB (9.7 MiB private) | 25.1 MiB (9.5 MiB private), 8 tunnels | 227–428 MiB |

The daemon figures are its resident set size (RSS), tray included; the 10 tunnels share one SSH session and carried a little traffic. The UI figure is the proportional set size (PSS, shared libraries counted once) of `tunneler-ui` plus its WebKit (Linux) or WebView2 (Windows) child processes, with the window open; the UI process exits when the window is closed. Linux: release build 0.1.0, Debian 13, WebKitGTK 2.54, UI under Xvfb with software rendering (likely an overestimate of a desktop with a GPU). Windows: release build 0.1.0, daemon working set (the RSS equivalent; Task Manager shows the private figure), the same with no tunnel and with 8 tunnels (within noise); the UI figure ranges from the private memory to the summed working sets of `tunneler-ui` and its six `msedgewebview2` processes (Windows has no PSS), `tunneler-ui` itself being 5.6 MiB private.

## Layout

| Path | Role |
|---|---|
| `crates/ssh-config` | `ssh_config` parser and resolver (OpenSSH semantics) |
| `crates/known-hosts` | `known_hosts` reader/writer |
| `crates/engine` | Sessions, auth, forwards, states, metrics, store |
| `crates/ipc` | Daemon ↔ clients protocol, client and server |
| `crates/tunneler` | `tunneler` binary: daemon, CLI, tray |
| `crates/test-support` | In-process SSH server and fixtures shared by the integration tests |
| `app/` | `tunneler-ui`: Tauri 2 (`src-tauri`) + React 19 (`src`) |

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml` (installed automatically by `rustup`).
- Node and pnpm versions pinned by `.mise.toml` (`mise install`).
- To build the UI on Linux, the Tauri 2 system dependencies:

  ```sh
  sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

The daemon crates do not need these: `app/src-tauri` is excluded from the workspace `default-members`.

## Development

```sh
# Rust (daemon crates)
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked

# Front end
cd app
pnpm install
pnpm check      # oxlint + oxfmt --check + tsc
pnpm test       # Vitest + Testing Library + axe
pnpm tauri dev  # run the UI (needs the Linux deps above)

# UI Rust crate (after `pnpm build`, which produces app/dist)
cargo clippy -p tunneler-ui --all-targets --locked -- -D warnings
```

### Releasing

1. Bump the version in the root `Cargo.toml` (`[workspace.package]`), `app/src-tauri/tauri.conf.json` and `app/package.json`, then refresh `Cargo.lock` with `cargo update -w`.
2. Tag the commit `vX.Y.Z` and push the tag.

The `release` workflow checks that the three versions and the tag agree, builds the Windows zip, the Linux tar.gz and the AppImage, and creates a draft GitHub release with these files and a `SHA256SUMS`. A manual run (`workflow_dispatch`) builds the same packages as run artifacts without releasing.

## Usage

```sh
tunneler                 # start the daemon (or ask the running one to show its window)
tunneler up dev          # start every tunnel named or tagged "dev" (starts the daemon if needed)
tunneler status          # runtime state, one tunnel per line (--json for scripts)
tunneler down --all
tunneler quit            # asks for confirmation when tunnels are active (--yes to skip)
```

Tunnels live in `tunnels.toml` (`~/.config/tunneler/` on Linux, `%APPDATA%\tunneler\` on Windows), reloaded on change. `tunneler --help` lists every command, the environment variables and the exit codes.
