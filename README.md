# Tunneler

Portable SSH tunnel manager (Windows + Linux, macOS without regressions): a light headless daemon with an embedded SSH client and a tray icon, plus an on-demand Tauri UI.

> Status: early development. See [`docs/PLAN.md`](docs/PLAN.md) for the design and [`docs/TASKS.md`](docs/TASKS.md) for progress.

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

## Usage

```sh
tunneler                 # start the daemon (or ask the running one to show its window)
tunneler up dev          # start every tunnel named or tagged "dev" (starts the daemon if needed)
tunneler status          # runtime state, one tunnel per line (--json for scripts)
tunneler down --all
tunneler quit            # asks for confirmation when tunnels are active (--yes to skip)
```

Tunnels live in `tunnels.toml` (`~/.config/tunneler/` on Linux, `%APPDATA%\tunneler\` on Windows), reloaded on change. `tunneler --help` lists every command, the environment variables and the exit codes.
