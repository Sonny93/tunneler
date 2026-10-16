# Tunneler — Tâches

Checklist d'exécution du [plan](./PLAN.md). Cocher au fil de l'eau ; ajouter une note sous la tâche quand la réalité diverge du plan (et mettre à jour `PLAN.md` si une décision change).

Légende : `[ ]` à faire · `[x]` fait · `[~]` en cours · `[-]` abandonné (avec raison)

**Définition de « fait » pour toute tâche UI/CLI** : utilisable au clavier seul, noms accessibles corrects, aucune info portée par la couleur seule, `axe` sans violation (UI) / sortie correcte avec `NO_COLOR=1` et hors TTY (CLI). Voir [PLAN §8](./PLAN.md#8-accessibilité).

---

## Phase 0 — Bootstrap ✔
- [x] 0.1 Workspace Cargo (`crates/ssh-config`, `crates/known-hosts`, `crates/engine`, `crates/ipc`, `crates/tunneler`), edition 2024, `rust-toolchain.toml`
  - Toolchain épinglé `1.99.0`. `app/src-tauri` est membre du workspace mais **exclu de `default-members`** : `cargo build/test/clippy` à la racine ne demandent pas les libs système webkit2gtk ; le crate UI se construit avec `-p tunneler-ui` (après `pnpm build`, `generate_context!` embarque `app/dist`).
  - Crates vides (doc `//!` seulement), aucune dépendance : chaque phase ajoute les siennes.
- [x] 0.2 Lints workspace (`[workspace.lints]` clippy pedantic sélectif), `rustfmt.toml`
  - Pedantic en warn, sauf `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc` ; `unwrap_used`/`expect_used`/`dbg_macro`/`todo` en warn (unwrap/expect autorisés en tests via `clippy.toml`) ; `unsafe_code = "deny"` (lever localement avec justification si besoin, ex. ACL named pipe). CI : `-D warnings`.
- [x] 0.3 App Tauri 2 dans `app/` (React 19 + TS + Vite + UnoCSS + `@minimalstuff/ui`), pnpm, Vitest + Testing Library + `vitest-axe`
  - `tunneler-ui` (bin), identifiant `com.minimalstuff.tunneler`, CSP restrictive, `bundle.active = false` jusqu'à la phase 13, capability `core:default` seule. Icônes provisoires générées depuis `app/src-tauri/app-icon.svg` (dossiers android/ios supprimés).
  - `vitest-axe` 0.1.0 fonctionne avec Vitest 5, mais ses types visent l'ancien namespace `Vi` → augmentation de `Assertion` dans `app/src/vitest_matchers.d.ts`.
  - Pas encore de `@tauri-apps/api` (phase 6).
- [x] 0.6 Outillage front via `@minimalstuff/tooling` : `.editorconfig`, `oxlint.config.ts` (`minimalstuffPreset({ react: true })` + plugin `jsx-a11y`), `oxfmt.config.ts`, scripts `lint` / `format` / `typecheck`
  - Vérifié : `jsx-a11y/alt-text`, `react/jsx-key` et `typescript/no-floating-promises` se déclenchent (le plugin ajouté n'écrase pas ceux du preset). Script `check` = lint + format:check + typecheck.
  - `.editorconfig` racine = preset + sections `*.rs`/`*.toml` (4 espaces) et `*.yml` (2 espaces). `.mise.toml` : node 24, pnpm 12, actionlint.
- [x] 0.4 CI GitHub Actions (Linux/Windows/macOS) : fmt, clippy, test, build UI
  - Jobs `rust` (fmt sur Linux, clippy, test) et `app` (deps Tauri apt sur Linux, `pnpm check` + `pnpm test` sur Linux, `pnpm build` puis clippy + build de `tunneler-ui` partout). Validé par `actionlint` uniquement : **jamais exécutée sur GitHub** à ce stade.
- [x] 0.5 `.gitignore`, `README.md`, `CLAUDE.md` projet (commandes, conventions, pointeur vers ce plan)

**Reste ouvert** : compilation de `app/src-tauri` non vérifiée en local (libs webkit2gtk-4.1 absentes de la machine de dev) — première vérification réelle = premier run CI, ou après installation des deps listées dans le README.

## Phase 1 — Crate `ssh-config`
- [ ] 1.1 Lexer : commentaires, `key value` / `key=value`, guillemets, échappements, insensibilité à la casse des clés
- [ ] 1.2 Modèle typé des directives utiles (Hostname, Port, User, IdentityFile, IdentitiesOnly, IdentityAgent, ProxyJump, ProxyCommand, LocalForward, RemoteForward, DynamicForward, ForwardX11, ServerAliveInterval/CountMax, StrictHostKeyChecking, UserKnownHostsFile, GlobalKnownHostsFile, HashKnownHosts, PreferredAuthentications, ConnectTimeout, AddKeysToAgent…) + conservation brute des inconnues
- [ ] 1.3 `Host` : motifs, wildcards `*` `?`, négation `!`
- [ ] 1.4 `Match` : `all`, `host`, `originalhost`, `user`, `localuser`, `exec`, `final`, `canonical`, négation
- [ ] 1.5 `Include` : chemins relatifs à `~/.ssh`, globs, récursion bornée, détection de cycles
- [ ] 1.6 Sémantique first-match-wins + directives cumulatives (IdentityFile, *Forward, CertificateFile, SendEnv)
- [ ] 1.7 Tokens `%h %p %r %u %n %d %C %L %l %i %%` + `~` + variables d'environnement `${VAR}`
- [ ] 1.8 API : `parse_file`, `list_hosts` (alias concrets, hors wildcards), `resolve(alias) -> ResolvedConfig`
- [ ] 1.9 Erreurs typées avec fichier + ligne ; directives non supportées remontées en warnings
- [ ] 1.10 Tests unitaires + tests différentiels contre `ssh -G` sur un corpus

## Phase 2 — Crate `known-hosts`
- [ ] 2.1 Parser : hostnames, `[host]:port`, wildcards, négation, entrées hashées `|1|salt|hash`
- [ ] 2.2 Marqueurs `@cert-authority`, `@revoked`
- [ ] 2.3 Sources multiples (`UserKnownHostsFile`, `GlobalKnownHostsFile`) depuis `ResolvedConfig`
- [ ] 2.4 Vérification : `Match` / `Unknown` / `Changed { line }` / `Revoked`
- [ ] 2.5 Ajout d'entrée (hashée si `HashKnownHosts`), écriture atomique
- [ ] 2.6 Tests sur fichiers générés par `ssh-keygen -H` / `ssh-keyscan`

## Phase 3 — Engine : connexion & auth
- [ ] 3.1 Abstractions : `CredentialPrompter`, `HostKeyVerifier`, `Clock`
- [ ] 3.2 Serveur SSH russh in-process pour les tests d'intégration (fixtures clés, users)
- [ ] 3.3 Transport TCP direct (ConnectTimeout, AddressFamily)
- [ ] 3.4 `ProxyJump` (chaîne multiple) via `direct-tcpip` + `connect_stream`
- [ ] 3.5 `ProxyCommand` via stdio d'un process enfant
- [ ] 3.6 Auth agent : `SSH_AUTH_SOCK`/`IdentityAgent` (Unix), named pipe OpenSSH + Pageant (Windows)
- [ ] 3.7 Auth clés (`IdentityFile`, passphrase via prompter, RSA hash via `best_supported_rsa_hash`)
- [ ] 3.8 Auth keyboard-interactive + password
- [ ] 3.9 Vérification clé d'hôte (known-hosts + `StrictHostKeyChecking`), refus dur si changée/révoquée
- [ ] 3.10 Keepalive depuis la config résolue

## Phase 4 — Engine : forwards locaux & cycle de vie
- [ ] 4.1 `SessionManager` : une session par alias, refcount, délai de grâce avant fermeture
- [ ] 4.2 Forward local : listener → `direct-tcpip` par connexion, copie bidirectionnelle
- [ ] 4.3 Machine à états des tunnels + événements de changement
- [ ] 4.4 Reconnexion : backoff exponentiel + jitter, plafond configurable
- [ ] 4.5 Santé de la cible : passive (`ConnectFailed`) + sonde active optionnelle
- [ ] 4.6 Métriques (connexions, octets, dernière activité) + logs ring buffer par tunnel
- [ ] 4.7 Store TOML : modèle, défauts (`listen_address`, `target_port = listen_port`), validation, écriture atomique, migrations, rechargement à chaud
- [ ] 4.8 Toggle par tag
- [ ] 4.9 Tests d'intégration : toggle isolé, reconnexion après coupure, port déjà pris

## Phase 5 — IPC, daemon, CLI
- [ ] 5.1 Types du protocole (serde) + versionnage + handshake
- [ ] 5.2 Serveur IPC (`interprocess` tokio), socket `0600` / named pipe ACL utilisateur, `PeerCreds`
- [ ] 5.3 Client IPC réutilisable (UI + CLI)
- [ ] 5.4 Daemon : single instance (bind = verrou, reclaim socket orphelin), 2ᵉ instance → `ui.show`
- [ ] 5.5 CLI : `up`, `down`, `status`, `list`, `open`, `quit` (sortie lisible + `--json`)
- [ ] 5.6 Logs fichier tournant (`tracing`)
- [ ] 5.7 CLI accessible : `NO_COLOR`, `TERM=dumb`, `--color auto|always|never`, détection TTY (pas de couleur/spinner/réécriture hors TTY), symboles + mots avec repli ASCII, sortie ligne à ligne sans tableaux en caractères de dessin
- [ ] 5.8 CLI : erreurs sur stderr avec cause + action suggérée, codes de sortie documentés, `--yes` pour les confirmations, aucun prompt hors TTY, `--help` avec exemples
- [ ] 5.9 Tests de sortie CLI (snapshots TTY / non-TTY / `NO_COLOR` / `--json`)

## Phase 6 — UI Tauri MVP
- [ ] 6.1 Pont Tauri ↔ IPC (commandes + relais des événements), spawn du daemon si absent
- [ ] 6.2 Store front (état alimenté par événements), design tokens, thème clair/sombre
- [ ] 6.3 Liste des tunnels groupée par host, switch, badges d'état et santé
- [ ] 6.4 Sidebar tags (toggle d'ensemble, compteurs) + hosts
- [ ] 6.5 Création/édition : port cible lié au port d'écoute jusqu'à modification, bind par défaut surchargeable, tags
- [ ] 6.6 Écran settings (dont bind par défaut local/remote)
- [ ] 6.7 Modales : passphrase/mot de passe, clé d'hôte inconnue, alerte clé changée
- [ ] 6.8 Quitter Tunneler : confirmation si tunnels actifs ; fermer la fenêtre = UI seule
- [ ] 6.9 Tests composants (Vitest + Testing Library, requêtes par rôle) + `axe` sur chaque composant/écran, échec CI sur violation
- [ ] 6.10 Design tokens vérifiés : contraste texte ≥ 4.5:1 et composants/focus ≥ 3:1 dans les deux thèmes, support `forced-colors`
- [ ] 6.11 Navigation clavier : ordre de tabulation, focus visible, roving tabindex dans la liste, raccourcis + aide `?`, cibles ≥ 24×24 px
- [ ] 6.12 Région `aria-live` pour les changements d'état (throttlée), focus géré dans les modales et retour au déclencheur
- [ ] 6.13 `prefers-reduced-motion`, zoom 200 % sans scroll horizontal
- [ ] 6.14 Passe manuelle : clavier seul, NVDA (Windows), Orca (Linux), contraste élevé Windows — résultats notés ici
- [ ] 6.15 Composants app absents de la lib : drawer, toasts + annonceur `aria-live`, liste navigable, badge d'état (icône + texte)
- [ ] 6.16 Audit `axe` + clavier des composants `@minimalstuff/ui` utilisés ; écarts corrigés dans la lib (issue/PR upstream)

## Phase 7 — Tray
- [ ] 7.1 `tray-icon` (feature `ksni` Linux), boucle d'événements par OS
- [ ] 7.2 Icône selon l'état global (forme distincte par état, pas seulement la couleur) + tooltip textuel, menu tags cochables avec état en texte + Ouvrir + Quitter
- [ ] 7.3 Lancement de `tunneler-ui` à la demande, focus si déjà ouverte
- [ ] 7.4 Quitter depuis le tray → confirmation via UI si tunnels actifs
- [ ] 7.5 Détection d'absence de StatusNotifierHost (GNOME) + message

## Phase 8 — Tunnels externes
- [ ] 8.1 `PortScanner` (`listeners` + `sysinfo`) avec cadence liée à la présence d'une UI
- [ ] 8.2 Parsing de ligne de commande `ssh` (`-L`/`-R`/`-D`, formes collées/séparées, host cible)
- [ ] 8.3 États `External` (tunnel réutilisé) et `Conflict`
- [ ] 8.4 Actions : adopter, kill, ignorer, importer
- [ ] 8.5 Section UI « Externes non gérés »
- [ ] 8.6 Test : tunnel lancé par `ssh -N -L` détecté puis adopté

## Phase 9 — Forwards remote & dynamic
- [ ] 9.1 Remote : `tcpip_forward`, callback `forwarded-tcpip`, `cancel_tcpip_forward`
- [ ] 9.2 Dynamic : serveur SOCKS5 (`fast-socks5`) → `direct-tcpip`
- [ ] 9.3 UI : sélecteur de type, champs adaptés
- [ ] 9.4 Tests d'intégration

## Phase 10 — X11
- [ ] 10.1 Parser `Xauthority` local + résolution du display (TCP / socket Unix)
- [ ] 10.2 Ancre X11 par host (`request_x11` cookie factice, lecture du `DISPLAY` distant)
- [ ] 10.3 Canaux entrants : substitution du cookie dans le setup X11 (ou auth vide)
- [ ] 10.4 Lanceurs X11 par host
- [ ] 10.5 UI : display distant + copier `export DISPLAY`, lanceurs, avertissement sécurité
- [ ] 10.6 Validation manuelle : VcXsrv (Windows) + Xorg/Xwayland (Linux)

## Phase 11 — Confort
- [ ] 11.1 Import des `LocalForward`/`RemoteForward`/`DynamicForward` de `~/.ssh/config`
- [ ] 11.2 Import par collage (script PS, liste de ports, specs `-L a:b:c`)
- [ ] 11.3 Palette de commandes `Ctrl+K`
- [ ] 11.4 Panneau logs live + métriques dans le drawer
- [ ] 11.5 Ouvrir dans le navigateur, copier la commande `ssh` équivalente
- [ ] 11.6 Vue « config résolue » d'un host (équivalent `ssh -G`)

## Phase 12 — Démarrage & secrets
- [ ] 12.1 Lancement au login (registre `Run` Windows, XDG autostart Linux)
- [ ] 12.2 `start_on_launch_tags` + `restore_on_launch` (`state.toml`)
- [ ] 12.3 Cache de passphrase en mémoire (`zeroize`) + trousseau OS optionnel (`keyring`)

## Phase 13 — Packaging & release
- [ ] 13.1 Windows : zip portable (`tunneler.exe` + `tunneler-ui.exe`), icônes, métadonnées
- [ ] 13.2 Linux : tar.gz (daemon) + AppImage (UI), dépendances documentées (webkit2gtk-4.1)
- [ ] 13.3 Mesures RAM (daemon idle, daemon 10 tunnels, UI ouverte) documentées dans le README
- [ ] 13.4 Workflow de release CI
- [ ] 13.5 Audit accessibilité final (WCAG 2.2 AA, checklist PLAN §8) sur Windows et Linux, résultats documentés
