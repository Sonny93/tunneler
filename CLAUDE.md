# Tunneler

SSH tunnel manager: Rust daemon (embedded `russh` client, tray, IPC) + on-demand Tauri 2 / React 19 UI.

## Source of truth

- [`docs/PLAN.md`](docs/PLAN.md): design and **locked decisions (§1)**. Do not re-litigate them; if one must change, update the plan and say why.
- [`docs/TASKS.md`](docs/TASKS.md): phased checklist. Tick tasks as they land and add a note under a task when reality diverges from the plan.
- Accessibility (PLAN §8) is part of "done" for every UI/CLI task, not a later polish pass.

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

UI Rust crate: `cargo clippy -p tunneler-ui --all-targets --locked -- -D warnings`. Requires `app/dist` (run `pnpm build` first, `tauri::generate_context!` embeds it) and, on Linux, the webkit2gtk-4.1 dev packages listed in the README.

CI (`.github/workflows/ci.yml`) runs the same commands on Linux, Windows and macOS. Validate workflow edits with `actionlint`.

## Conventions

### Rust
- Edition 2024, toolchain pinned in `rust-toolchain.toml`. Every crate sets `[lints] workspace = true`; lints live in the root `Cargo.toml` (clippy pedantic, `unsafe_code = "deny"`, no `unwrap`/`expect` outside tests).
- Unit tests co-located (`#[cfg(test)]`), integration tests in each crate's `tests/`, using an in-process russh server (never a system `sshd`).
- `engine` depends on abstractions (`CredentialPrompter`, `HostKeyVerifier`, `Clock`…), not on IPC or UI.
- Add dependencies in the phase that needs them, not ahead of time.

### Front end (`app/`)
- File names in `snake_case`, imports through the `#/*` alias (`./src/*`).
- Lint/format presets from `@minimalstuff/tooling` (+ oxlint `jsx-a11y` plugin); components from `@minimalstuff/ui`, utilities from UnoCSS. Accessibility gaps found in `@minimalstuff/ui` are fixed upstream, not worked around here.
- Tests: Vitest + Testing Library, query by role/accessible name first, and an `axe` check (`vitest-axe`) for every component and screen.

### Language
Code, comments, commits and these files are in English. `docs/PLAN.md` and `docs/TASKS.md` are in French. User-facing UI strings are French (`<html lang="fr">`).
