# Tunneler

SSH tunnel manager: Rust daemon (embedded `russh` client, tray, IPC) + on-demand Tauri 2 / React 19 UI.

## Source of truth

- [`docs/roadmap/decisions.md`](docs/roadmap/decisions.md): the **locked design decisions**. Do not re-litigate them; if one must change, update that page and say why.
- [`docs/roadmap/index.md`](docs/roadmap/index.md): what is shipped, what is next, what needs testing, known limitations. Update it as work lands: move items from Next to Shipped, keep Needs testing and Known limitations current.
- [`docs/contributing/accessibility.md`](docs/contributing/accessibility.md): the accessibility rules. They are part of "done" for every UI/CLI task, not a later polish pass.
- Every docs page exists in English and in French (`docs/fr/` mirrors `docs/`): update both in the same change.

## Commands

Repo root (Rust; `app/src-tauri` is excluded from `default-members`, so these need no GUI system deps):

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

`app/` (front end, run with pnpm from `.mise.toml`):

```sh
pnpm check   # oxlint (type-aware) + oxfmt --check + tsc --noEmit
pnpm test    # vitest run
pnpm format  # lint:fix + oxfmt
pnpm build   # vite build → app/dist
```

On Windows, the UI crate also needs the daemon staged as its sidecar: `cargo build -p tunneler`, then copy `target/debug/tunneler.exe` to `app/src-tauri/binaries/tunneler-x86_64-pc-windows-msvc.exe` (gitignored; CI does the same).

UI Rust crate: `cargo clippy -p tunneler-ui --all-targets --locked -- -D warnings`. Requires `app/dist` (run `pnpm build` first, `tauri::generate_context!` embeds it) and, on Linux, the webkit2gtk-4.1 dev packages listed in the README.

`docs/` (VitePress site, English at the root and French under `fr/`, run with pnpm):

```sh
pnpm dev     # local server with hot reload
pnpm build   # static site → docs/.vitepress/dist (fails on dead links)
```

Pitfalls:
- `cargo build --release -p tunneler-ui` without `tauri/custom-protocol` builds in `cfg(dev)` and loads `devUrl` (blank window without `pnpm dev`). Build the release UI with `pnpm tauri build` only.
- `pnpm tauri build` rewrites `app/src-tauri/Cargo.toml` (`features = []`): never commit that rewrite.
- Updater artifacts need the signing key: without `TAURI_SIGNING_PRIVATE_KEY`, `pnpm tauri build` bundles then fails. Build locally with `pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}' …` (only the release workflow signs).
- Headless (Xvfb), WebKitGTK never starts without a D-Bus session and reports no error: run the UI under `dbus-run-session`.

CI (`.github/workflows/ci.yml`) runs the same commands on Linux and Windows (macOS is not maintained: no CI, cfg-gated code kept as is). Validate workflow edits with `actionlint`.

## Conventions

### Rust
- Edition 2024, toolchain pinned in `rust-toolchain.toml`. Every crate sets `[lints] workspace = true`; lints live in the root `Cargo.toml` (clippy pedantic, `unsafe_code = "deny"`, no `unwrap`/`expect` outside tests).
- Unit tests co-located (`#[cfg(test)]`), integration tests in each crate's `tests/`, using the in-process russh server from `crates/test-support` (never a system `sshd`). `test-support` must never depend on `engine` (a dev-dependency cycle duplicates the engine types); engine-trait fakes live in `crates/engine/tests/support/`.
- `ipc` never depends on `engine`: wire DTOs are the protocol contract (golden JSON tests), the daemon maps between them.
- Windows-only code is linted locally with `CC_x86_64_pc_windows_gnu=gcc AR_x86_64_pc_windows_gnu=ar cargo clippy --workspace --exclude tunneler-ui --all-targets --target x86_64-pc-windows-gnu -- -D warnings`.
- `engine` depends on abstractions (`CredentialPrompter`, `HostKeyVerifier`, `Clock`…), not on IPC or UI.
- Add dependencies in the phase that needs them, not ahead of time.

### Front end (`app/`)
- File names in `snake_case`, imports through the `#/*` alias (`./src/*`).
- Lint/format presets from `@minimalstuff/tooling` (+ oxlint `jsx-a11y` plugin); components from `@minimalstuff/ui`, utilities from UnoCSS. Accessibility gaps found in `@minimalstuff/ui` are fixed upstream, not worked around here.
- Tests: Vitest + Testing Library, query by role/accessible name first, and an `axe` check (`vitest-axe`) for every component and screen.

### Language
Code, comments, commits and these files are in English. User-facing strings are written in English and translated to French: Lingui in `app/` (macros, catalogs in `app/src/locales/`, run `pnpm i18n:extract` and fill every French `msgstr` in the same change), a per-language `match` in the daemon for the tray and notifications. The CLI is English only. The display language is `settings.language` (`system` follows the OS).
