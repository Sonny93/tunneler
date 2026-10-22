# Développement

## Prérequis

- La chaîne d'outils Rust fixée par `rust-toolchain.toml`. `rustup` l'installe au premier usage.
- Node et pnpm aux versions fixées par `.mise.toml` : lancez `mise install`.
- Pour construire l'interface sous Linux, les paquets système de Tauri 2 :

  ```sh
  sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

Les crates du daemon n'en ont pas besoin : `app/src-tauri` est exclu des `default-members` du workspace.

## Organisation du dépôt

| Chemin | Rôle |
|---|---|
| `crates/ssh-config` | Analyseur et résolveur de `ssh_config`, avec la sémantique d'OpenSSH. |
| `crates/known-hosts` | Lecteur et écrivain de `known_hosts`. |
| `crates/engine` | Sessions, authentification, redirections, états, métriques, le store, X11 et les ouvertures. |
| `crates/ipc` | Le protocole entre le daemon et ses clients, avec le client et le serveur. |
| `crates/tunneler` | Le programme `tunneler` : daemon, ligne de commande et icône de notification. |
| `crates/test-support` | Un serveur SSH en processus et des fixtures partagés par les tests d'intégration. |
| `app/` | `tunneler-ui` : Tauri 2 dans `src-tauri`, React 19 dans `src`. |
| `docs/` | Ce site, construit avec VitePress. |

Les règles qui séparent les crates :

- `engine` dépend d'abstractions (`CredentialPrompter`, `HostKeyVerifier`, `Clock`…), jamais de l'IPC ni de l'interface.
- `ipc` ne dépend jamais d'`engine`. Les types du protocole sont le contrat, avec des tests JSON de référence, et le daemon fait la conversion.
- `test-support` ne dépend jamais d'`engine`. Les faux des traits du moteur vivent dans `crates/engine/tests/support/`.

Les raisons des choix principaux sont sur la page [Décisions de conception](../roadmap/decisions).

## Construire et tester

Depuis la racine du dépôt, pour les crates Rust :

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Les tests unitaires sont à côté du code, dans des modules `#[cfg(test)]`. Les tests d'intégration sont dans le dossier `tests/` de chaque crate et utilisent le serveur en processus de `crates/test-support`, jamais un `sshd` du système. Les lints sont réglés dans le `Cargo.toml` racine : clippy pedantic, `unsafe_code` interdit, et ni `unwrap` ni `expect` hors des tests.

Le code qui ne se construit que sous Windows se lint depuis Linux avec :

```sh
CC_x86_64_pc_windows_gnu=gcc AR_x86_64_pc_windows_gnu=ar cargo clippy --workspace --exclude tunneler-ui --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

Pour le front end, dans `app/` :

```sh
pnpm install
pnpm check      # oxlint, oxfmt --check et tsc --noEmit
pnpm test       # Vitest, Testing Library et axe
pnpm format     # lint:fix et oxfmt
pnpm build      # vite build dans app/dist
pnpm tauri dev  # lance l'interface ; demande les paquets Linux ci-dessus
```

Chaque composant et chaque écran a un test `axe`. Les textes de l'interface sont en anglais, avec un catalogue français dans `app/src/locales`.

Le crate Rust de l'interface a besoin de `app/dist` : lancez d'abord `pnpm build` :

```sh
cargo clippy -p tunneler-ui --all-targets --locked -- -D warnings
```

Sous Windows, le crate de l'interface a aussi besoin du daemon comme sidecar. Lancez `cargo build -p tunneler`, puis copiez `target/debug/tunneler.exe` vers `app/src-tauri/binaries/tunneler-x86_64-pc-windows-msvc.exe`. Ce dossier est ignoré par git, et la CI fait pareil.

### Pièges

- `cargo build --release -p tunneler-ui` sans `tauri/custom-protocol` construit en `cfg(dev)` et charge `devUrl`, soit une fenêtre blanche sans `pnpm dev`. Construisez l'interface de production avec `pnpm tauri build`.
- `pnpm tauri build` réécrit `app/src-tauri/Cargo.toml` (`features = []`). Ne le commitez pas.
- Sans `TAURI_SIGNING_PRIVATE_KEY`, `pnpm tauri build` crée les paquets puis échoue. Pour construire en local, passez `--config '{"bundle":{"createUpdaterArtifacts":false}}'`.
- Sans écran, WebKitGTK ne démarre jamais sans session D-Bus et n'affiche aucune erreur. Lancez l'interface sous `dbus-run-session`.

La CI lance les mêmes commandes sous Linux et sous Windows. Validez les modifications d'un workflow avec `actionlint`.

## Accessibilité

Un changement de l'interface ou de la ligne de commande n'est pas terminé tant qu'il ne respecte pas les [règles d'accessibilité](./accessibility).

## Documentation

Le site est dans `docs/`. Les pages anglaises sont à la racine et les pages françaises les reflètent sous `docs/fr/`, avec les mêmes noms de fichiers. Ajoutez une page dans les deux langues et dans les deux barres latérales de `docs/.vitepress/config.ts`.

```sh
cd docs
pnpm install
pnpm dev      # un serveur local avec rechargement
pnpm build    # un lien mort fait échouer la construction
```

Quand les textes de l'interface changent, vérifiez le guide avec les libellés anglais de `app/src/locales/en/messages.po` et les libellés français de `app/src/locales/fr/messages.po`. Le workflow `.github/workflows/docs.yml` construit le site à chaque changement sous `docs/` et le publie depuis `main`.

## Publier une version

Les versions sont créées avec [release-it](https://github.com/release-it/release-it), depuis `app/` :

```sh
cd app
GITHUB_TOKEN=$(gh auth token) pnpm release              # interactif, incrément recommandé
GITHUB_TOKEN=$(gh auth token) pnpm release --dry-run    # aperçu seulement
GITHUB_TOKEN=$(gh auth token) pnpm release minor        # ou `patch`, ou une version explicite
```

L'incrément vient des Conventional Commits depuis le dernier tag et reste en 0.x : un changement incompatible incrémente la version mineure, tout le reste la version de correctif. release-it :

1. met à jour la version dans le `Cargo.toml` racine (`[workspace.package]`), `app/src-tauri/tauri.conf.json` et `app/package.json`, et rafraîchit `Cargo.lock` ;
2. écrit `CHANGELOG.md` ;
3. commite `chore(release): vX.Y.Z`, pose le tag et pousse ;
4. crée une version GitHub en brouillon, avec le changelog comme notes.

Le tag déclenche le workflow `release`. Il vérifie que les trois versions et le tag concordent, construit l'installeur et le zip Windows ainsi que le `.deb`, le tar.gz et l'AppImage Linux, et écrit `latest.json`, le manifeste de mise à jour avec les signatures de l'installeur et de l'AppImage. Il joint ensuite le tout, avec un fichier `SHA256SUMS`, à la version en brouillon et la publie. Un lancement manuel (`workflow_dispatch`) construit les mêmes paquets comme artefacts du run, sans publier.

Pour la première version, taguez le 0.1.0 existant tel quel : `GITHUB_TOKEN=$(gh auth token) pnpm release --no-increment`.

### Clé de signature

Les mises à jour sont signées avec une clé minisign. Sa moitié publique est dans `app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). La moitié privée et son mot de passe sont les secrets de dépôt `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. La construction en a besoin car `bundle.createUpdaterArtifacts` est activé.

Gardez une copie des deux hors de GitHub. Un programme installé n'accepte que les mises à jour signées avec la clé fournie avec lui : si la clé est perdue, chaque copie installée doit être réinstallée à la main.
