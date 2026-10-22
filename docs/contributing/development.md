# Development

## Prerequisites

- The Rust toolchain pinned by `rust-toolchain.toml`. `rustup` installs it on first use.
- Node and pnpm at the versions pinned by `.mise.toml`: run `mise install`.
- To build the UI on Linux, the Tauri 2 system packages:

  ```sh
  sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

The daemon crates do not need them: `app/src-tauri` is left out of the workspace's `default-members`.

## Repository layout

| Path | Role |
|---|---|
| `crates/ssh-config` | `ssh_config` parser and resolver, with OpenSSH semantics. |
| `crates/known-hosts` | `known_hosts` reader and writer. |
| `crates/engine` | Sessions, authentication, forwards, states, metrics, the store, X11 and the openers. |
| `crates/ipc` | The protocol between the daemon and its clients, with the client and the server. |
| `crates/tunneler` | The `tunneler` program: daemon, command line and tray. |
| `crates/test-support` | An in-process SSH server and fixtures shared by the integration tests. |
| `app/` | `tunneler-ui`: Tauri 2 in `src-tauri`, React 19 in `src`. |
| `docs/` | This site, built with VitePress. |

The rules that keep the crates apart:

- `engine` depends on abstractions (`CredentialPrompter`, `HostKeyVerifier`, `Clock`…), never on IPC or the UI.
- `ipc` never depends on `engine`. The wire types are the protocol contract, with golden JSON tests, and the daemon maps between them.
- `test-support` never depends on `engine`. Fakes of the engine's traits live in `crates/engine/tests/support/`.

The reasons behind the main choices are on the [Design decisions](../roadmap/decisions) page.

## Build and test

From the repository root, for the Rust crates:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Tests sit next to the code in `#[cfg(test)]` modules. Integration tests are in each crate's `tests/` folder and use the in-process server of `crates/test-support`, never a system `sshd`. Lints are set in the root `Cargo.toml`: clippy pedantic, `unsafe_code` denied, and no `unwrap` or `expect` outside tests.

Code that only builds on Windows is linted from Linux with:

```sh
CC_x86_64_pc_windows_gnu=gcc AR_x86_64_pc_windows_gnu=ar cargo clippy --workspace --exclude tunneler-ui --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

For the front end, in `app/`:

```sh
pnpm install
pnpm check      # oxlint, oxfmt --check and tsc --noEmit
pnpm test       # Vitest, Testing Library and axe
pnpm format     # lint:fix and oxfmt
pnpm build      # vite build into app/dist
pnpm tauri dev  # run the UI; needs the Linux packages above
```

Every component and screen has an `axe` test. The UI strings are English with a French catalog under `app/src/locales`.

The UI's Rust crate needs `app/dist`, so run `pnpm build` first:

```sh
cargo clippy -p tunneler-ui --all-targets --locked -- -D warnings
```

On Windows, the UI crate also needs the daemon as its sidecar. Run `cargo build -p tunneler`, then copy `target/debug/tunneler.exe` to `app/src-tauri/binaries/tunneler-x86_64-pc-windows-msvc.exe`. That folder is git-ignored, and CI does the same.

### Pitfalls

- `cargo build --release -p tunneler-ui` without `tauri/custom-protocol` builds in `cfg(dev)` and loads `devUrl`, which is a blank window without `pnpm dev`. Build the release UI with `pnpm tauri build`.
- `pnpm tauri build` rewrites `app/src-tauri/Cargo.toml` (`features = []`). Do not commit that.
- Without `TAURI_SIGNING_PRIVATE_KEY`, `pnpm tauri build` bundles and then fails. To build locally, pass `--config '{"bundle":{"createUpdaterArtifacts":false}}'`.
- Headless, WebKitGTK never starts without a D-Bus session and prints no error. Run the UI under `dbus-run-session`.

CI runs the same commands on Linux and Windows. Validate changes to a workflow with `actionlint`.

## Accessibility

A change to the UI or the command line is not done until it meets the [accessibility rules](./accessibility).

## Documentation

The site is in `docs/`. English pages are at the root and the French ones mirror them under `docs/fr/`, with the same file names. Add a page to both languages and to both sidebars in `docs/.vitepress/config.ts`.

```sh
cd docs
pnpm install
pnpm dev      # a local server with live reload
pnpm build    # a dead link fails the build
```

When the UI wording changes, check the guide against the English labels in `app/src/locales/en/messages.po` and the French ones in `app/src/locales/fr/messages.po`. The workflow `.github/workflows/docs.yml` builds the site on every change under `docs/` and publishes it from `main`.

## Releasing

Releases are cut with [release-it](https://github.com/release-it/release-it) from `app/`:

```sh
cd app
GITHUB_TOKEN=$(gh auth token) pnpm release              # interactive, recommended bump
GITHUB_TOKEN=$(gh auth token) pnpm release --dry-run    # preview only
GITHUB_TOKEN=$(gh auth token) pnpm release minor        # or `patch`, or an explicit version
```

The bump comes from the Conventional Commits since the last tag and stays in 0.x: a breaking change bumps the minor version, everything else bumps the patch version. release-it then:

1. bumps the version in the root `Cargo.toml` (`[workspace.package]`), `app/src-tauri/tauri.conf.json` and `app/package.json`, and refreshes `Cargo.lock`;
2. writes `CHANGELOG.md`;
3. commits `chore(release): vX.Y.Z`, tags it and pushes;
4. creates a draft GitHub release with the changelog as notes.

The tag triggers the `release` workflow. It checks that the three versions and the tag agree, builds the Windows installer and zip and the Linux `.deb`, tar.gz and AppImage, and writes `latest.json`, the update manifest with the signatures of the installer and the AppImage. It then attaches everything with a `SHA256SUMS` file to the draft release and publishes it. A manual run (`workflow_dispatch`) builds the same packages as run artifacts without releasing.

For the first release, tag the existing 0.1.0 as it is: `GITHUB_TOKEN=$(gh auth token) pnpm release --no-increment`.

### Signing key

Updates are signed with a minisign key. Its public half is in `app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). The private half and its password are the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets. The build needs them because `bundle.createUpdaterArtifacts` is on.

Keep a copy of both outside GitHub. An installed program accepts only updates signed with the key it shipped with, so if the key is lost, every installed copy has to be reinstalled by hand.
