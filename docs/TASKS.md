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

**Vérifié a posteriori (2026-10-16)** : une fois les deps webkit2gtk-4.1 installées, `app/src-tauri` passe `cargo clippy -p tunneler-ui --all-targets -- -D warnings` et `cargo build -p tunneler-ui` sous Linux. Fenêtre jamais lancée (`pnpm tauri dev`) à ce stade.

## Phase 1 — Crate `ssh-config` ✔
- [x] 1.1 Lexer : commentaires, `key value` / `key=value`, guillemets, échappements, insensibilité à la casse des clés
  - Portage de `strdelim` + `argv_split` d'OpenSSH : un token commençant par `#` termine la ligne (commentaire de fin), sauf pour `ProxyCommand` qui garde tout.
- [x] 1.2 Modèle typé des directives utiles (Hostname, Port, User, IdentityFile, IdentitiesOnly, IdentityAgent, ProxyJump, ProxyCommand, LocalForward, RemoteForward, DynamicForward, ForwardX11, ServerAliveInterval/CountMax, StrictHostKeyChecking, UserKnownHostsFile, GlobalKnownHostsFile, HashKnownHosts, PreferredAuthentications, ConnectTimeout, AddKeysToAgent…) + conservation brute des inconnues
  - Modélisé en plus : CertificateFile, ForwardX11Trusted, ForwardAgent, HostKeyAlias, Pubkey/Password/KbdInteractiveAuthentication, NumberOfPasswordPrompts, ConnectionAttempts, AddressFamily, BatchMode, ExitOnForwardFailure, GatewayPorts, ClearAllForwardings, TCPKeepAlive, Compression, SendEnv, SetEnv, IgnoreUnknown, Tag.
  - Mots-clés OpenSSH connus non modélisés → `ResolvedConfig::other` (bruts, sans validation), silencieux. Mots-clés inconnus (hors `IgnoreUnknown`) → warning, pas erreur.
- [x] 1.3 `Host` : motifs, wildcards `*` `?`, négation `!`
  - `Host` : motifs séparés par espaces, **sensible à la casse**, pas de virgule. `Match host/originalhost` : listes à virgules, insensibles à la casse.
- [x] 1.4 `Match` : `all`, `host`, `originalhost`, `user`, `localuser`, `exec`, `final`, `canonical`, négation
  - + `tagged` (avec `Tag`). `localnetwork`, `version`, `sessiontype`, `command` → warning, ne matchent jamais (même niés).
  - `Match exec` via `$SHELL -c` (repli `/bin/sh`), stdio nul ; `cmd /C` sous Windows (non vérifié).
- [x] 1.5 `Include` : chemins relatifs à `~/.ssh`, globs, récursion bornée, détection de cycles
  - Profondeur max 16 (`READCONF_MAX_DEPTH`). Un Include dans un bloc inactif est parsé mais rien ne s'applique (NEVERMATCH) ; l'état du bloc englobant est restauré après chaque fichier.
  - Tokens/env expansés avec l'état courant ; `${VAR}` indéfinie → include ignoré (comme OpenSSH) + `WarningKind::IncludeSkipped`.
- [x] 1.6 Sémantique first-match-wins + directives cumulatives (IdentityFile, *Forward, CertificateFile, SendEnv)
  - Passe finale (déclenchée par `Match final`/`canonical`, même dans un include inactif) : les lignes `Host` matchent alors le **hostname résolu** (minuscule sauf IP), plus l'alias.
  - `ProxyJump none` ne bloque pas un `ProxyCommand` ultérieur (comportement du binaire 10.0p2, différent de la lecture du source).
  - `CanonicalizeHostname` ≠ `no` → warning, traité comme `no`.
- [x] 1.7 Tokens `%h %p %r %u %n %d %C %L %l %i %%` + `~` + variables d'environnement `${VAR}`
  - + `%j` (hôte du dernier saut) et `%k`. `%C` = SHA-1 hex de `%l%h%p%r%j`. Jeu de tokens autorisé par directive ; token invalide = erreur typée **dès la résolution** (ssh ne valide `IdentityFile` qu'à la connexion).
  - `ResolvedConfig` contient les valeurs finales expansées (hostname, identity/certificate files, known hosts, agent, proxy command, sockets unix). `ProxyJump` non expansé (comme `ssh -G`).
- [x] 1.8 API : `parse_file`, `list_hosts` (alias concrets, hors wildcards), `resolve(alias) -> ResolvedConfig`
  - `resolve(&ConfigSources, alias, &impl LocalEnvironment) -> Result<Resolution { config, warnings }>`. `ConfigSources::{platform_default, user_only}` (équivalent `-F`). Fichier racine absent = vide (≠ `ssh -F absent`, fatal).
  - `LocalEnvironment` (env, user, home, hostname, uid, dossier config système, `Match exec`) ; `SystemEnvironment::new()` faillible (échoue tôt).
  - `pub mod differential_support` caché (`#[doc(hidden)]`) : expose l'expander de tokens et le parser ProxyJump au test différentiel uniquement, pas une API stable.
- [x] 1.9 Erreurs typées avec fichier + ligne ; directives non supportées remontées en warnings
- [x] 1.10 Tests unitaires + tests différentiels contre `ssh -G` sur un corpus
  - 436 tests unitaires ; corpus de 33 fichiers / 81 cas (fichier, alias), cas d'erreur inclus (les deux côtés doivent échouer).
  - Oracle = **OpenSSH 10.0p2 Debian** : le test est `cfg(target_os = "linux")` et s'ignore si `ssh -V` ≠ `OpenSSH_10.0`. Patches Debian neutralisés dans la comparaison (`ForwardX11Trusted` par défaut `yes`, `BatchMode` → keepalive 300) ; on garde les défauts upstream.
  - CI : job dédié `ssh-config-differential` dans un conteneur `debian:trixie`, rouge si le test est ignoré. Jamais exécuté sur GitHub à ce stade.

**Lacunes connues** : ports nommés (`getservbyname`), `~user/`, contrôles de permissions des fichiers (`SSHCONF_CHECKPERM`), `ForwardAgent no` suivi d'un chemin. Comportements Windows (chemins `%PROGRAMDATA%`, `cmd /C`, uid 0, globs) non vérifiés.

## Phase 2 — Crate `known-hosts` ✔
- [x] 2.1 Parser : hostnames, `[host]:port`, wildcards, négation, entrées hashées `|1|salt|hash`
  - Lignes invalides (base64, champs manquants, hash, algorithme du blob ≠ type déclaré) → warning + ligne ignorée, jamais d'échec du fichier. Sel hashé = exactement 20 octets. CRLF toléré.
- [x] 2.2 Marqueurs `@cert-authority`, `@revoked`
  - Un seul marqueur ; inconnu ou dupliqué → ligne invalide. `@revoked` est **filtré par les motifs d'hôte** (comme `load_hostkeys`), pas global.
- [x] 2.3 Sources multiples (`UserKnownHostsFile`, `GlobalKnownHostsFile`) depuis `ResolvedConfig`
  - Écart : le crate ne dépend ni de `ssh-config` ni de `russh`. `KnownHosts::load(user_files, global_files)` ; c'est l'engine (3.9) qui mappe `ResolvedConfig` (`*_known_hosts_files`, `host_key_alias`, `hash_known_hosts`).
- [x] 2.4 Vérification : `Match` / `Unknown` / `Changed { line }` / `Revoked`
  - `Verdict::{Trusted, Unknown, Changed { known }, Revoked}`. Précédence (OpenSSH `check_hostkeys_by_key_or_type`) : révoqué > clé égale dans n'importe quel fichier > **toute autre entrée, quel que soit l'algorithme → `Changed`** > `Unknown`. Volontairement strict : pas de prompt « nouvel hôte » pour un algorithme non enregistré (vecteur MITM).
  - Nom de recherche : `HostKeyAlias` tel quel (sans port), sinon `[host]:port` si port ≠ 22, avec repli sur le nom nu quand le résultat est `Unknown` (comme `check_host_key`).
  - `KnownHosts::known_algorithms(lookup)` pour que l'engine ordonne les algorithmes de clé d'hôte comme `order_hostkeyalgs` (évite les faux `Changed`).
  - Certificats : `PresentedKey::Certificate { signing_authority }` — l'engine valide le certificat, ce crate dit seulement si la CA est de confiance pour l'hôte.
- [x] 2.5 Ajout d'entrée (hashée si `HashKnownHosts`), écriture atomique
  - `append_entry(path, lookup, key, Hashing::{Plain, Hashed})` : temp + fsync + rename, permissions conservées, nouveau fichier `0600` / dossier `0700`, suit les symlinks, n'altère jamais les lignes existantes. Noms contenant espaces ou `@?*#[]|'"\!,` refusés.
- [x] 2.6 Tests sur fichiers générés par `ssh-keygen -H` / `ssh-keyscan`
  - 130 tests unitaires + 9 d'intégration : `check` comparé à `ssh-keygen -F` (fichiers plain, multi-motifs, wildcards, négation, `-H`), empreintes comparées à `ssh-keygen -l`, nos entrées écrites retrouvées par `ssh-keygen -F`.
  - `ssh-keyscan` non utilisé (nécessite un sshd) : même format de ligne que les `.pub` testés.

**Lacunes connues** : révocation de la clé du certificat elle-même (seule la CA est vérifiée) ; marqueurs testés uniquement en unitaire (`ssh-keygen -F` les ignore) ; Windows non vérifié.

## Phase 3 — Engine : connexion & auth ✔
- [x] 3.1 Abstractions : `CredentialPrompter`, `HostKeyVerifier`, `Clock`
  - Traits `#[async_trait]` utilisés via `Arc<dyn …>` : `CredentialPrompter` (passphrase / password / keyboard-interactive, `None` = annulé, secrets `Zeroizing<String>`), `HostKeyVerifier` (`decide_unknown` → `AcceptAndSave | AcceptOnce | Reject`, `report_rejected` = notification seule), `HostConfigResolver` (+ `SshConfigResolver`), `AgentLocator` (+ `SystemAgentLocator`).
  - **`Clock` reporté à la phase 4** (aucun usage avant la reconnexion) pour éviter une abstraction prématurée.
  - Point d'entrée : `Connector::new(ConnectorDependencies { resolver, prompter, verifier, agent_locator })`, `Connector::connect(alias) -> SshSession` (`alias`, `resolved`, `jump_chain`, `open_direct_tcpip`, `disconnect`).
- [x] 3.2 Serveur SSH russh in-process pour les tests d'intégration (fixtures clés, users)
  - `crates/engine/tests/support/` : serveur configurable (méthodes, users, partial success, `direct-tcpip`, enregistre cibles et clés offertes), agent russh sur socket Unix temporaire, environnement/prompter/verifier scriptés. Clés RSA pré-générées dans `tests/fixtures/` (keygen RSA en debug = jusqu'à 10 s), ed25519 générées à l'exécution.
  - Helper `[[bin]] tcp-bridge` (stdin↔TCP) dans le crate engine pour tester ProxyCommand sans `nc` ; construit aussi par `cargo build`, jamais livré.
- [x] 3.3 Transport TCP direct (ConnectTimeout, AddressFamily)
  - `ConnectTimeout` borne chaque connexion TCP **et** le handshake ; le chrono est suspendu pendant un prompt de clé d'hôte. `ConnectionAttempts` = tours complets sur la liste d'adresses, pause 1 s.
- [x] 3.4 `ProxyJump` (chaîne multiple) via `direct-tcpip` + `connect_stream`
  - Sémantique OpenSSH (`-J`) : le 1ᵉʳ saut utilise sa config complète (son propre ProxyJump/ProxyCommand → récursion), les sauts suivants et la cible ignorent leurs proxys. user/port de la spec surchargent ceux du saut. Profondeur max 8 + détection de cycle par alias. Erreur `ConnectError::Jump { hop, source }`.
  - La session cible possède ses sauts ; `disconnect()` ferme cible puis sauts en ordre inverse. Pas encore de partage d'un saut entre plusieurs cibles (à voir avec le `SessionManager`, 4.1).
- [x] 3.5 `ProxyCommand` via stdio d'un process enfant
  - `$SHELL -c "exec <cmd>"` (repli `/bin/sh`), `cmd /C` sous Windows (approximation : Win32-OpenSSH fait un `posix_spawnp` direct, non vérifié). stderr → `tracing::warn`, enfant tué et récolté à la fermeture/drop.
- [x] 3.6 Auth agent : `SSH_AUTH_SOCK`/`IdentityAgent` (Unix), named pipe OpenSSH + Pageant (Windows)
  - Clés agent d'abord, puis fichiers non déjà offerts (dédup par clé publique, sans prompt de passphrase si l'agent a la clé). `IdentitiesOnly` filtre l'agent sur les `IdentityFile` configurés. Agent injoignable → fichiers. Échec de signature par l'agent → erreur (russh laisserait la requête en suspens).
  - Branche Windows (named pipe + Pageant, natifs dans russh 0.64) écrite mais **jamais compilée ni exécutée**. `AddKeysToAgent` non implémenté.
- [x] 3.7 Auth clés (`IdentityFile`, passphrase via prompter, RSA hash via `best_supported_rsa_hash`)
  - Formats via russh : OpenSSH, PKCS#8, PEM/PKCS#5, PuTTY `.ppk` (seul OpenSSH testé de bout en bout). Détection de chiffrement ajoutée pour PEM/PPK. Essais de passphrase = `NumberOfPasswordPrompts` (comme OpenSSH). Lecture/déchiffrement en `spawn_blocking`.
  - Certificats utilisateur (`CertificateFile` + `authenticate_openssh_cert`) câblés mais sans test d'intégration.
  - Fichiers d'identité absents : toujours en debug (`ResolvedConfig` ne distingue pas défauts et fichiers explicites).
- [x] 3.8 Auth keyboard-interactive + password
  - Ordre publickey → keyboard-interactive → password, filtré par `PreferredAuthentications` et `*Authentication no`, respect de `remaining_methods`/`partial_success` (multi-facteur testé). Annulation d'un prompt = méthode suivante ; erreur du prompter = échec. Keyboard-interactive plafonné à 10 tours. `BatchMode yes` = aucun prompt.
- [x] 3.9 Vérification clé d'hôte (known-hosts + `StrictHostKeyChecking`), refus dur si changée/révoquée
  - `Changed`/`Revoked` refusés quel que soit `StrictHostKeyChecking` (même `no`). `Unknown` : `yes` → refus, `ask` → prompt (refus en `BatchMode`), `accept-new`/`no` → enregistré sans prompt. `AcceptOnce` mémorisé en RAM pour la vie du daemon, consulté seulement en mode `ask`. Échec d'écriture du known_hosts → connexion refusée.
  - Algorithmes de clé d'hôte ordonnés selon `KnownHosts::known_algorithms` (testé : known_hosts RSA seul + serveur ed25519+RSA → `Trusted`).
  - **Certificats d'hôte non supportés** : russh 0.64 peut les recevoir mais la validation serait à écrire ; un certificat reçu est refusé (`UnsupportedKey`), `@cert-authority` inutilisable pour l'instant.
- [x] 3.10 Keepalive depuis la config résolue
  - `ServerAliveInterval` (0 = désactivé) / `ServerAliveCountMax` → `keepalive_interval` / `keepalive_max` de russh.

**Choix techniques** : russh 0.64 sans features par défaut, backend `ring` (pas d'aws-lc → pas de cmake sous Windows), `flate2`, `rsa`. 132 tests engine (76 unitaires + 56 intégration), suite en quelques secondes. Rien de vérifié sous Windows.

## Phase 4 — Engine : forwards locaux & cycle de vie ✔
- [x] 4.1 `SessionManager` : une session par alias, refcount, délai de grâce avant fermeture
  - `acquire(alias)` → bail libéré au drop ; acquisitions concurrentes = **une seule connexion en vol** (donc un seul prompt). Session morte (`SshSession::closed()`, y compris perte d'un saut) → retirée, prochaine acquisition reconnecte. Settings appliqués aux nouvelles sessions seulement.
  - Les sessions de saut restent possédées par leur cible : deux cibles derrière le même bastion ouvrent chacune leur connexion au bastion.
- [x] 4.2 Forward local : listener → `direct-tcpip` par connexion, copie bidirectionnelle
  - Bind **avant** la session (port pris → `Failed { PortInUse }` sans SSH). Listener conservé pendant `Reconnecting` (connexions acceptées fermées aussitôt). Adresse d'origine réelle transmise au serveur.
- [x] 4.3 Machine à états des tunnels + événements de changement
  - Un acteur par tunnel ; bus `broadcast` d'`EngineEvent::{TunnelChanged, TunnelMetrics, TunnelLog, ConfigReloaded, ConfigInvalid}`. `ConfigReloaded` aussi émis après create/update/delete.
  - **Écart PLAN** : `Active` sans compteur de connexions (sinon un `TunnelChanged` par connexion) ; le compteur vit dans `TunnelMetrics.active_connections`, throttlé. `External`/`Conflict` arrivent en phase 8.
- [x] 4.4 Reconnexion : backoff exponentiel + jitter, plafond configurable
  - `min(initial × 2^(n−1), max)` avec equal jitter, fonction pure. Classification exhaustive (match sans joker) : transport, timeouts, perte de session et erreurs d'E/S du handshake → retry ; clé d'hôte, auth, config, chaîne de sauts, prompt, agent, violations de protocole → `Failed` sans retry (pas de tempête de prompts).
  - `Clock` (reporté de 3.1) introduit ici, uniquement pour les horodatages visibles ; les attentes passent par `tokio::time`.
  - Keepalive effectif = `ServerAlive*` de la config résolue s'ils sont définis (`ResolvedConfig` les expose désormais en `Option`), sinon `settings.keepalive_*` ; `ServerAliveInterval 0` explicite désactive. Testé : serveur muet → perte de session → `Reconnecting`.
- [x] 4.5 Santé de la cible : passive (`ConnectFailed`) + sonde active optionnelle
  - Sonde active toutes les 30 s si `settings.health_probe` **et** `Engine::set_health_probing(true)` (le daemon l'activera quand une UI est connectée).
- [x] 4.6 Métriques (connexions, octets, dernière activité) + logs ring buffer par tunnel
  - `TunnelMetrics { active_connections, total_connections, bytes_in, bytes_out, last_activity }`, remis à zéro à chaque démarrage, événement ≤ 1/s par tunnel et seulement si changé. Ring buffer de 500 entrées par tunnel.
- [x] 4.7 Store TOML : modèle, défauts (`listen_address`, `target_port = listen_port`), validation, écriture atomique, migrations, rechargement à chaud
  - `toml_edit` : commentaires et mise en forme préservés lors des écritures de l'app (tunnels appariés par `id`). Clés inconnues = erreur avec ligne/colonne. Tunnel sans `id` → id généré et fichier réécrit (par l'engine au démarrage et au rechargement). Version plus récente que l'app → refus, fichier jamais réécrit.
  - Validation exhaustive (toutes les erreurs) : ids/noms dupliqués (noms insensibles à la casse), ports, conflits d'écoute effectifs (sans détecter `0.0.0.0` vs `127.0.0.1`), tags, settings.
  - Watcher : dossier parent, debounce 200 ms, ignore nos propres écritures (hash), fichier invalide → dernier document valide conservé. `Store` synchrone (à appeler via `run_blocking` depuis l'async).
  - Helper d'écriture atomique dupliqué depuis `known-hosts` (2ᵉ copie : la prochaine justifie un crate partagé).
- [x] 4.8 Toggle par tag
- [x] 4.9 Tests d'intégration : toggle isolé, reconnexion après coupure, port déjà pris
  - 58 tests runtime (+ unitaires : backoff, classification, diff de définitions, throttling, ring buffer), stables sur 13 exécutions dont 4 en parallèle. Les assertions négatives (pas de retry, pas de sonde) reposent sur de courtes fenêtres de silence.

**Lacunes connues** : partage des sauts entre cibles ; Windows/macOS non vérifiés. Rechargement : un changement `None` → valeur par défaut explicite redémarre le tunnel (comparaison des champs bruts).

## Phase 5 — IPC, daemon, CLI ✔
- [x] 5.1 Types du protocole (serde) + versionnage + handshake
  - `ipc` ne dépend pas d'`engine` : DTO propres (camelCase, enums taggés `kind`, horodatages en **epoch ms**), mapping dans le daemon. Contrat figé par des tests JSON « golden » (`crates/ipc/tests/protocol_contract.rs`).
  - `hello { protocolVersion, clientKind, clientVersion, subscribeEvents, canPrompt }` ; version différente → `protocol_mismatch` puis fermeture ; timeout de handshake 10 s. Codes d'erreur fermés (`invalid_request`, `unknown_method`, `not_found`, `validation_failed`, `tunnels_running`, `prompt_not_pending`, `ui_unavailable`…). Prompts corrélés par `promptId`, `prompt.resolved` pour fermer les dialogues ailleurs.
  - Pas encore d'`external.*`, `hosts.*`, `x11.*`, `import.*` ni d'états `External`/`Conflict` : ajouts additifs dans leurs phases (changement de protocole à versionner).
- [x] 5.2 Serveur IPC (`interprocess` tokio), socket `0600` / named pipe ACL utilisateur, `PeerCreds`
  - Unix : `0600` posé avant le `bind` (fchmod), uid du pair vérifié (fail closed). Sans `$XDG_RUNTIME_DIR` (et sous macOS) : `<tmp>/tunneler-<uid>/tunneler.sock` en `0700`, refusé si le dossier n'est pas privé.
  - Windows : pipe `tunneler-<user>`, SDDL `D:P(A;;GA;;;OW)(A;;GA;;;SY)` sans `unsafe` ; pas de contrôle de pair, l'ACL est la frontière. **Jamais exécuté.**
  - Requêtes concurrentes par connexion, file sortante bornée (client lent = déconnecté), lignes ≤ 1 Mio.
- [x] 5.3 Client IPC réutilisable (UI + CLI)
  - `IpcClient` typé, corrélation par id, `take_events()` à appeler avant de partager le client (pont Tauri).
- [x] 5.4 Daemon : single instance (bind = verrou, reclaim socket orphelin), 2ᵉ instance → `ui.show`
  - `tunneler` sans argument : daemon au premier plan, ou `ui.show` si un daemon répond (répond `ui_unavailable` tant qu'il n'y a pas d'UI). Options `--socket`, `--config`, `--ssh-config` (équivalent `ssh -F`) + variables `TUNNELER_*`.
  - Broker de prompts : envoyés aux clients `canPrompt`, premier qui répond gagne, timeout 120 s, aucun répondeur → annulation/refus (seam isolée pour lancer l'UI en phase 7). Sonde de santé active tant qu'un client `ui` est connecté.
  - `settings.update` → `Engine::update_settings` (même verrou et même store que le CRUD).
- [x] 5.5 CLI : `up`, `down`, `status`, `list`, `open`, `quit` (sortie lisible + `--json`)
  - `up` démarre le daemon détaché s'il est absent, attend Active/Failed (`--timeout`, `--no-wait`) et répond aux prompts sur le TTY (passphrase masquée, clé d'hôte `y`/`o`/`N`). Résolution : nom exact (insensible à la casse) puis tag, ambiguïté → `--tag` (sauf si le tag ne désigne que ce même tunnel).
  - `list` passe par le daemon (exit 3 sans daemon). `down` n'attend pas la fermeture des ports.
- [x] 5.6 Logs fichier tournant (`tracing`)
  - Daemon : `TUNNELER_LOG` (défaut `info`), rotation quotidienne, 7 fichiers dans `<data local>/tunneler/logs`, + stderr si TTY. CLI silencieuse sauf `TUNNELER_LOG`.
- [x] 5.7 CLI accessible : `NO_COLOR`, `TERM=dumb`, `--color auto|always|never`, détection TTY (pas de couleur/spinner/réécriture hors TTY), symboles + mots avec repli ASCII, sortie ligne à ligne sans tableaux en caractères de dessin
  - Heures de santé affichées en **UTC** (pas d'API d'heure locale dans std) — à revoir si besoin d'une dépendance tz.
- [x] 5.8 CLI : erreurs sur stderr avec cause + action suggérée, codes de sortie documentés, `--yes` pour les confirmations, aucun prompt hors TTY, `--help` avec exemples
  - Codes : 0 ok, 1 erreur (dont « déjà lancé »), 2 usage, 3 daemon absent, 4 échec partiel (`up`), 5 confirmation impossible (hors TTY sans `--yes`), 6 timeout.
- [x] 5.9 Tests de sortie CLI (snapshots TTY / non-TTY / `NO_COLOR` / `--json`)
  - 182 tests unitaires tunneler + 16 E2E sur le vrai binaire (daemon auto-lancé, echo à travers les tunnels, `NO_COLOR` sans séquences ANSI, quit refusé hors TTY, socket orphelin récupéré après `kill -9`), stables sur 6 exécutions.
  - Serveur SSH de test extrait dans `crates/test-support` (dépend de russh seulement, jamais d'`engine`) ; les fakes des traits engine restent dans `crates/engine/tests/support/`.
  - Prompts interactifs vérifiés une fois à la main sous pty (`script`), pas de test conservé.

**Vérifications** : 1348 tests verts. Lint Windows de tout le workspace (hors `tunneler-ui`) propre en cross-clippy `x86_64-pc-windows-gnu` (avec `CC/AR` pointant sur gcc/ar faute de mingw), mais rien d'exécuté sous Windows ni macOS (détachement, named pipe, signaux, `open`).

## Phase 6 — UI Tauri MVP (reste : NVDA/contraste élevé 6.14, zoom réel 6.13, correctifs upstream 6.16)
- [x] 6.1 Pont Tauri ↔ IPC (commandes + relais des événements), spawn du daemon si absent
  - `app/src-tauri` : une commande Tauri par méthode IPC (`connect_daemon`, `snapshot`, `list_hosts`, CRUD, start/stop/restart, tags, settings, réponses aux prompts, `quit_daemon`) ; événements `daemon-event` (l'`EventMessage` tel quel) et `daemon-disconnected`. Erreurs sérialisées `{ code, message, data? }` : codes IPC + `daemon_unreachable` / `disconnected`. Connexion pilotée par le front, idempotente (tentatives sérialisées, garde de génération contre une reconnexion concurrente). Pas de reconnexion automatique : écran « Daemon injoignable » + bouton *Réessayer*.
  - Binaire daemon : `TUNNELER_DAEMON_BINARY`, sinon `tunneler` à côté de `tunneler-ui`, sinon `PATH`. La logique « lancer détaché et attendre » est sortie du CLI vers `ipc::ensure_daemon_running` (partagée CLI/UI).
  - **Ajout protocole** : `hosts.list` (alias concrets de `ssh_config`, via `HostConfigResolver::list_hosts`) pour le formulaire de création. Additif, `PROTOCOL_VERSION` inchangé (un daemon plus ancien répond `unknown_method`).
  - `quit_daemon` réussi → `app.exit(0)`. Secrets des prompts en `Zeroizing` dès la désérialisation.
  - Smoke test réel sous Xvfb : l'UI lance le daemon et lit la liste. **Sans session D-Bus, la webview WebKitGTK ne démarre jamais (aucun `WebKitWebProcess`, pas d'erreur)** : `dbus-run-session` requis en environnement headless. Rendu jamais vu par un humain à ce stade.
- [x] 6.2 Store front (état alimenté par événements), design tokens, thème clair/sombre
  - Front dépend de l'interface `DaemonGateway` (implémentation Tauri + fake de test). zustand : `daemon_store` (réducteur d'événements pur), `ui_store` (panneau, filtre, aide). `config.reloaded` → relecture du snapshot. Types TS écrits à la main en miroir des DTO Rust (pas de validation runtime : le pont désérialise déjà avec serde).
  - Tokens sémantiques (`src/theme/tokens.ts`) exposés en variables CSS `--tn-*` + couleurs Uno `tn-*` ; thème clair/sombre via `ThemeToggle` de la lib (classe `dark`, `system` = `prefers-color-scheme`).
- [x] 6.3 Liste des tunnels groupée par host, switch, badges d'état et santé
  - Switch nommé « Tunnel Adonis, 3333 vers nova:3333, actif » ; endpoint et métriques avec flèches visuelles masquées et équivalent texte lu. Menu d'actions par ligne : Modifier, Redémarrer, Supprimer (confirmation). Après suppression, focus sur la ligne suivante/précédente ou sur « Créer un tunnel ».
- [x] 6.4 Sidebar tags (toggle d'ensemble, compteurs) + hosts
  - Switch de tag coché si **tous** ses tunnels tournent ; basculer démarre le tag sinon l'arrête. Filtre actif : `aria-current` + gras + barre + icône. Filtre vers un tag/host disparu → retour à « Tous ».
- [x] 6.5 Création/édition : port cible lié au port d'écoute jusqu'à modification, bind par défaut surchargeable, tags
  - Host : champ libre + `datalist` des alias (`hosts.list`). Tags séparés par des virgules. Type toujours `local` (sélecteur en 9.3). Erreurs `validation_failed` du daemon dans un résumé `role="alert"` qui prend le focus.
- [x] 6.6 Écran settings (dont bind par défaut local/remote)
  - Panneau latéral (drawer) plutôt qu'un écran. Seuls les réglages effectifs sont exposés (adresses, keepalive, délai de grâce, reconnexion, sonde) ; `restore_on_launch`, `start_on_launch_tags`, `launch_at_login`, `x11_display` sont conservés tels quels jusqu'aux phases 10/12. Durées saisies en secondes.
- [x] 6.7 Modales : passphrase/mot de passe, clé d'hôte inconnue, alerte clé changée
  - Une modale à la fois (file FIFO) ; `prompt.resolved` ferme la modale sans répondre. Fermer/Échap = annuler (credential) ou refuser (clé d'hôte). Alerte clé changée/révoquée : non fermable par Échap, seul bouton « J'ai compris », aucun contournement.
- [x] 6.8 Quitter Tunneler : confirmation si tunnels actifs ; fermer la fenêtre = UI seule
  - `quit(false)` → si `tunnels_running`, confirmation « Tout arrêter et quitter » → `quit(true)`. `quit.requested` (tray, phase 7) déclenche la même confirmation — le daemon ne l'émet pas encore.
- [x] 6.9 Tests composants (Vitest + Testing Library, requêtes par rôle) + `axe` sur chaque composant/écran, échec CI sur violation
  - 72 fichiers / 557 tests front. CI : `pnpm test` existant suffit (un `axe` en échec fait échouer le test).
- [x] 6.10 Design tokens vérifiés : contraste texte ≥ 4.5:1 et composants/focus ≥ 3:1 dans les deux thèmes, support `forced-colors`
  - Ratios vérifiés par test unitaire (`tokens.test.ts`) dans les deux thèmes. `forced-colors` : bordure `CanvasText` sur les badges, focus en `Highlight`, icônes `forced-color-adjust: none` — jamais vu en contraste élevé réel.
- [x] 6.11 Navigation clavier : ordre de tabulation, focus visible, roving tabindex dans la liste, raccourcis + aide `?`, cibles ≥ 24×24 px
  - Roving à deux colonnes (switch / bouton d'actions) sur toute la liste, ↑/↓/Début/Fin ; gestion en phase de capture → ↓ n'ouvre plus le menu d'actions depuis son bouton (Entrée/Espace oui). Raccourcis `n`, `,`, `?` (désactivés dans les champs, avec Ctrl/Alt/Meta, panneau ou modale ouvert). Taille des cibles non mesurée dans un vrai rendu.
- [x] 6.12 Région `aria-live` pour les changements d'état (throttlée), focus géré dans les modales et retour au déclencheur
  - Changements regroupés sur 1 s (détail si ≤ 3 tunnels), rien au chargement initial ni pour la santé seule.
- [~] 6.13 `prefers-reduced-motion`, zoom 200 % sans scroll horizontal
  - Animations/transitions coupées sous `prefers-reduced-motion` ; mise en page empilée sous `md` (sidebar au-dessus). Zoom 200 % **non vérifié** dans une vraie webview.
- [~] 6.14 Passe manuelle : clavier seul, NVDA (Windows), Orca (Linux), contraste élevé Windows — résultats notés ici
  - **Linux fait (2026-10-16)** : vraie fenêtre GNOME/Xwayland, WebKitGTK 2.54.0, clavier synthétisé (XTest) + lecture de l'arbre AT-SPI (ce qu'Orca consomme ; Orca lui-même non lancé). Restent : NVDA, contraste élevé Windows, écoute Orca par un humain.
  - Conformes : ordre de tabulation (lien d'évitement → en-tête → sidebar → ligne active), roving ↑/↓/Début/Fin sur les deux colonnes, Espace sur un switch, modale clé d'hôte (« Accepter et enregistrer » écrit `known_hosts`), formulaire (libellés, requis, indices, focus initial et sur le premier champ invalide, port cible lié/délié), menu d'actions (élément désactivé sauté), confirmation de suppression en `alertdialog` focus sur « Annuler » puis focus sur la ligne voisine, fieldsets des réglages nommés, focus rendu à « Nouveau tunnel » après *Annuler*, badge d'échec + annonce live, fermer la fenêtre laisse le daemon.
  - Corrigés suite à la passe : `<ul>` en `list-none` exposés en simple panneau par WebKit (→ `role="list"`) ; compteurs lus « 0/4 actifs » (→ « 0 sur 4 actifs ») ; raison d'échec et cible injoignable absentes du switch (→ `aria-describedby`) ; titres « TAGS »/« HOSTS » exposés en capitales (CSS `uppercase` retiré) ; en-tête/pied du drawer exposés en landmarks anonymes (→ `div`) ; erreurs de champ persistantes après correction (→ effacées à l'édition) ; focus perdu après création (→ switch du nouveau tunnel, repli sur `main`) ; bouton « Close » sur la confirmation de suppression ; raccourcis inactifs quand le focus est sur un switch (`input` checkbox pris pour un champ éditable) ; **`ThemeToggle` fait planter le `WebKitWebProcess`** (View Transition, reproduit 2/2, OK sous `prefers-reduced-motion`) → transition désactivée.
  - Notés : le bouton du menu d'actions est exposé « combo box » (mapping WebKitGTK des boutons `aria-haspopup`) ; une authentification qui ne répond jamais (agent SSH qui attend une confirmation) laisse le tunnel en « Démarrage… » sans fin — pas de délai d'authentification côté moteur → tâche 11.7.
- [x] 6.15 Composants app absents de la lib : drawer, toasts + annonceur `aria-live`, liste navigable, badge d'état (icône + texte)
  - Drawer : dialogue modal en portail, focus piégé et restauré, pas d'`inert` sur le reste (couperait aussi les régions live). Toasts d'erreur `role="alert"` persistants ; info/succès 5 s, minuteur en pause au survol/focus.
- [~] 6.16 Audit `axe` + clavier des composants `@minimalstuff/ui` utilisés ; écarts corrigés dans la lib (issue/PR upstream)
  - Couvert par les tests (axe + clavier) des composants qui les utilisent. Écarts à remonter dans `minimalstuff/ui` (**rien d'ouvert upstream**) :
    - `Modal` force `role="dialog"` : pas d'`alertdialog` possible pour l'alerte clé changée (seul `ConfirmModal` l'utilise).
    - `Modal` n'expose pas `initialFocusRef` : sur « Hôte inconnu », le focus initial tombe sur « Copier l'empreinte » au lieu de « Refuser ».
    - `ModalFooter` avec des enfants recréés à chaque rendu (render prop) boucle à l'infini.
    - Libellés par défaut en anglais (`Close`, `Confirm`, `Cancel`) : l'app passe ses libellés partout.
    - `Input` sans prop `description` (contrairement à `Switch`) : indices reliés à la main via `aria-describedby` (fusionné correctement par la lib).
    - Erreur de champ `Input` rendue en `<p role="alert">` : WebKitGTK ne l'inclut pas dans la description calculée (la relation `described-by` est pourtant correcte), donc pas relue au focus ; et plusieurs alertes partent en même temps à la soumission. Préférer un texte non-live relié par `aria-describedby` + le résumé d'erreurs.
    - `ThemeToggle` : `document.startViewTransition` fait planter WebKitGTK 2.54 ; l'app passe `isTransitionEnabled={false}`. Détecter/éviter côté lib.
  - Vérifiés conformes : `aria-invalid` + erreur liée sur `Input`/`Switch`, focus restauré par `Modal`, raccourcis globaux désactivés pendant une modale.

## Phase 7 — Tray ✔ (validé sur GNOME ; Windows jamais exécuté, macOS sans tray)
- [x] 7.1 `tray-icon` (feature `ksni` Linux), boucle d'événements par OS
  - `tray-icon` 0.26 sans features par défaut : `ksni` sous Linux (D-Bus pur Rust, **aucune dépendance GTK** vérifiée par `cargo tree`, un seul `zbus` partagé), backend Win32 sous Windows. `TrayIcon` et les items muda sont `!Send` (`Rc`) → un thread std « tray » les possède sur chaque OS ; le reste du daemon lui envoie `Render(TrayView)` / `Shutdown`, les clics reviennent par un mpsc tokio (handlers globaux `MenuEvent` / `TrayIconEvent`).
  - Linux : pas de boucle d'événements (ksni a son propre thread). Windows : boucle `winit` 0.30 (`with_any_thread`), commandes en user events via `EventLoopProxy` — **seulement cross-linté, jamais exécuté**.
  - **Écart PLAN** : macOS n'a pas de tray (il exigerait la boucle sur le thread principal, occupé par tokio) ; statut `disabled`, rien d'autre ne change (« sans régression »).
  - `--no-tray` / `TUNNELER_NO_TRAY` (propagé au daemon lancé par la CLI) ; les tests E2E l'utilisent toujours, avec un `TUNNELER_UI_BINARY` inexistant, pour ne jamais ouvrir d'icône ni de fenêtre.
- [x] 7.2 Icône selon l'état global (forme distincte par état, pas seulement la couleur) + tooltip textuel, menu tags cochables avec état en texte + Ouvrir + Quitter
  - Icônes 32×32 dessinées en code (pas de fichiers), contour sombre : repos = anneau gris, OK = disque vert, dégradé = triangle ambre, erreur = losange rouge. Précédence : un `Failed` → erreur ; sinon reconnexion ou cible injoignable d'un tunnel lancé → dégradé ; sinon `Active`/`Starting` → OK ; sinon repos. Icônes définitives en phase 13.
  - Tooltip « Tunneler — 3 tunnels actifs, 1 en reconnexion » (+ « N en démarrage », « N en échec », « N cibles injoignables »). Menu : ligne de résumé désactivée (l'état global est aussi lisible au clavier/lecteur d'écran dans le menu), tags en cases à cocher « dev — 3 sur 5 actifs » (cochée si tous lancés, même règle et même formulation que la sidebar), « Ouvrir Tunneler », « Quitter ». Clic gauche sur l'icône = ouvrir. Menu reconstruit à chaque changement de vue (ids stables `tag:<nom>`, `open`, `quit`).
- [x] 7.3 Lancement de `tunneler-ui` à la demande, focus si déjà ouverte
  - `UiCoordinator` : une UI = une connexion `kind: ui`. `ui.show` (2ᵉ instance, tray) → événement **`show.requested`** aux UI connectées (le pont Tauri fait `unminimize`/`show`/`set_focus`), sinon lancement de `tunneler-ui` (`TUNNELER_UI_BINARY`, sinon à côté de `tunneler`, sinon `PATH` ; détaché, récolté, `TUNNELER_SOCKET` et `TUNNELER_DAEMON_BINARY` transmis). Pas de 2ᵉ lancement pendant 30 s tant que l'UI ne s'est pas connectée. Échec → `ui_unavailable`, et la CLI sort en 1 avec un indice.
  - Localisation des binaires compagnons factorisée dans `ipc::locate_companion_binary` (daemon → UI, UI → daemon).
  - Prompt sans répondeur (PLAN §3) : le daemon lance l'UI et attend un client `canPrompt`, le tout borné par les 120 s du prompt. Le pont Tauri demande l'attention (clignotement de la barre des tâches, sans voler le focus) sur une demande de secret ou de clé d'hôte quand la fenêtre n'a pas le focus.
  - `tunneler` sans argument qui devient le daemon ouvre aussi l'UI (lancement manuel = « montre-moi Tunneler ») ; `tunneler daemon` non.
  - Vérifié en vrai sous Xvfb + bus D-Bus privé : la 2ᵉ instance lance l'UI, qui se connecte (`kind=Ui`) ; une 3ᵉ invocation n'ouvre pas de 2ᵉ UI. Rendu de la fenêtre non capturé (capture `xwd` blanche) ; le focus réel n'est pas vérifiable sans gestionnaire de fenêtres. Rappel : un `cargo build -p tunneler-ui` sans `--features tauri/custom-protocol` charge `devUrl` (fenêtre blanche sans `pnpm dev`).
- [x] 7.4 Quitter depuis le tray → confirmation via UI si tunnels actifs
  - Aucun tunnel actif → arrêt direct. Sinon `quit.requested` aux UI connectées (qui affichent la confirmation existante et se mettent au premier plan), ou lancement de l'UI avec la confirmation délivrée à sa connexion — valable seulement pendant la fenêtre de lancement, jamais livrée à une UI ouverte plus tard. UI impossible à lancer → erreur loguée, le daemon continue (jamais d'arrêt des tunnels sans confirmation).
- [x] 7.5 Détection d'absence de StatusNotifierHost (GNOME) + message
  - `tray-icon` crée ksni avec `assume_sni_available(true)` et `IsStatusNotifierHostRegistered` vaut toujours vrai sous KDE et GNOME : le vrai signal est la présence d'un propriétaire du nom `org.kde.StatusNotifierWatcher`, suivi en continu (`NameOwnerChanged`, thread dédié, `zbus` bloquant).
  - **Ajout protocole** (additif, `PROTOCOL_VERSION` inchangé) : `SnapshotDto.tray` (`active` | `missingHost` | `failed { message }` | `disabled`) et l'événement `tray.changed`. L'UI affiche un bandeau (section titrée, non live, « Masquer » pour la session) pour `missingHost` (rouvrir via `tunneler`, extension GNOME AppIndicator) et `failed`. `status --json` contient aussi `tray`.
  - Vérifié en vrai sur bus privé avec un faux watcher (Python/Gio) piloté via `com.canonical.dbusmenu` : `missingHost` → `active` à l'arrivée du watcher → `missingHost` à son départ ; titre, tooltip, icône 32×32, menu, clic sur un tag (démarre puis arrête), « Quitter » avec/sans tunnel actif. **Validé à la main (2026-10-17) sur GNOME 48 + extension AppIndicator** (active après reconnexion de session ; installée en cours de session, elle reste `INACTIVE`) : icône affichée dans la barre, UI ouverte depuis le menu, « Quitter » avec tunnel actif → confirmation dans l'UI → arrêt complet. Restent : KDE, Windows.

**Vérifications** : fmt, clippy (workspace + `tunneler-ui` + cross-clippy Windows) et tous les tests verts (277 unitaires tunneler, 18 E2E CLI, 571 tests front). Rien d'exécuté sous Windows ni macOS.

## Phase 8 — Tunnels externes ✔ (Linux ; Windows jamais exécuté)
- [x] 8.1 `PortScanner` (`listeners` + `sysinfo`) avec cadence liée à la présence d'une UI
  - Trait `PortScanner` (scan bloquant + `terminate`) dans `engine`, implémentation `SystemPortScanner` : sockets TCP en écoute (`listeners` 0.6) + processus propriétaires et processus `ssh` (`sysinfo` 0.39, feature `system` seule ; aucune dépendance lourde). Processus identifié par pid **et** heure de démarrage : `terminate` refuse un pid réutilisé (`ProcessChanged`), envoie SIGTERM (repli `kill`, `TerminateProcess` sous Windows).
  - Passe de scan à la connexion d'une UI puis toutes les 5 s (`EngineOptions::port_scan_interval`) tant qu'une UI est connectée (`daemon/ui_presence.rs`, même suiveur que la sonde de santé), et au démarrage d'un tunnel dont le port est pris. Le daemon s'exclut lui-même. Erreur de scan → avertissement, états inchangés.
  - **Lacune** : sans UI connectée, aucun scan périodique → `tunneler status` n'affiche les externes non gérés que si une UI a scanné récemment ; les états External/Conflict restent ceux du dernier scan.
- [x] 8.2 Parsing de ligne de commande `ssh` (`-L`/`-R`/`-D`, formes collées/séparées, host cible)
  - Options façon getopt d'OpenSSH (groupées `-NfL…`, collées, séparées, `--`, arrêt à la destination). Specs analysées par `ssh-config` (nouveaux `parse_*_forward_argument`, sans expansion d'environnement). Destination réduite au host (`user@`, `ssh://…:port`, crochets IPv6). Ignorés : sockets Unix, `-R` dynamique, `-o LocalForward=…` (non interprété).
  - Correspondance tunnel ↔ forward : même type, même port d'écoute, adresses compatibles (sans bind = loopback, joker = tout, nom d'hôte autre que `localhost` = compatible faute de DNS), même port cible et host cible équivalent (`localhost`/`127.0.0.0/8`/`::1` identiques). **La destination ssh n'est pas comparée.**
- [x] 8.3 États `External` (tunnel réutilisé) et `Conflict`
  - `External { owner }`, `Conflict { owner? }` (owner absent = processus inconnu, ex. socket d'un autre utilisateur). Ni l'un ni l'autre ne compte comme « en cours » (quitter, tray, switch de tag). Observation appliquée seulement à un tunnel arrêté, External, Conflict ou `Failed { PortInUse }` ; port libéré → `Stopped`. *Arrêter* garde l'état observé. Démarrer sur un port pris → External/Conflict au lieu de `Failed { PortInUse }` (gardé si le scan échoue).
  - **Protocole v2** (`PROTOCOL_VERSION` 2 : nouvelles variantes d'état), `SnapshotDto.externals`, événement `externals.changed`, codes d'erreur `invalid_state` et `kill_failed`.
  - CLI : `⇄ provided by external ssh (pid N)` / `! port held by node (pid N)` ; `up` réussit (exit 0) si chaque cible est Active ou External ; un tunnel déjà External/Conflict avant `up` est considéré réglé (un nouvel essai ne change pas l'état, donc aucun événement). Bloc « Unmanaged ssh processes » dans `status`.
- [x] 8.4 Actions : adopter, kill, ignorer, importer
  - `external.adopt { id }` : termine le ssh, attend la libération de ses ports (sondage 100 ms, 5 s max → `kill_failed`) puis démarre **tous** les tunnels configurés qu'il fournissait (un même ssh fournit souvent plusieurs `-L`), renvoie leurs ids. `external.kill { pid }` / `external.ignore { pid }` : pid accepté seulement s'il est signalé (owner d'un External/Conflict ou externe non géré).
  - **Interprétation d'« Ignorer »** : le daemon cesse de signaler ce processus (identité pid + démarrage) jusqu'à son arrêt, non persisté — ses tunnels External repassent `Stopped`, il quitte la liste des non gérés ; un démarrage explicite rapporte quand même la vérité.
  - **Écart PLAN** : pas de méthode `external.import` ; l'UI ouvre le formulaire de création prérempli (nom, host = destination, ports, adresses) puis passe par `tunnel.create`, ce qui laisse nommer/étiqueter le tunnel. Le tunnel créé devient External au scan suivant et peut être adopté.
  - Pas de commandes CLI `adopt`/`kill`/`ignore` (suivi possible).
- [x] 8.5 Section UI « Externes non gérés »
  - Badges « Externe — fourni par ssh (pid N) » et « Conflit — port occupé par … » (icône + ton + texte, détail relié au switch par `aria-describedby`, annoncé dans la région live). Menu de ligne : *Adopter*, *Arrêter le processus externe…*, *Ignorer* (External) ; *Arrêter le processus…* (Conflict avec owner). Arrêt toujours confirmé (`ConfirmModal`, conséquences explicites).
  - Section `<section>` « Externes non gérés (N) » sous la liste, une entrée par processus (`h3` « ssh vers nova (pid N) »), forwards `-L 3333 → localhost:3333` avec équivalent texte, *Importer comme tunnel* (forwards locaux seulement avant la phase 9), *Arrêter le processus…*, *Ignorer* ; noms accessibles commençant par le texte visible (WCAG 2.5.3).
  - Pas d'entrée « Externes » dans la sidebar : la section suffit. Rendu réel non vérifié dans une webview.
- [x] 8.6 Test : tunnel lancé par `ssh -N -L` détecté puis adopté
  - Binaire de test `fake-ssh` (copié en `ssh` dans un dossier temporaire, il écoute sur ses `-L`) : détecté `External` avec le vrai pid, adopté → processus terminé, tunnel Active et echo à travers le serveur russh de test ; un `-L` hors config apparaît dans les externes. Plus ~22 tests runtime sur un scanner scripté.
  - Smoke manuel : `fake-ssh` réel + daemon + `tunneler up` → « provided by external ssh (pid N) », exit 0. Le vrai client OpenSSH n'a pas été utilisé (il exige un serveur).

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows) et tests verts (1644 tests Rust, 628 tests front). Rien d'exécuté sous Windows ni macOS (`listeners`/`sysinfo`/`TerminateProcess` non vérifiés).

## Phase 9 — Forwards remote & dynamic ✔ (Linux ; `curl --socks5` réel non exécuté)
- [x] 9.1 Remote : `tcpip_forward`, callback `forwarded-tcpip`, `cancel_tcpip_forward`
  - `RemoteForwardRegistry` par session SSH (créé par le connecteur, partagé entre `ClientHandler` et `SshSession`) : routage par port annoncé par le serveur ; si plusieurs forwards de la session partagent ce port, l'adresse (insensible à la casse) départage, sinon rejet (`AdministrativelyProhibited` + avertissement). Les serveurs ne rapportent pas tous `localhost` de la même façon, d'où le port seul quand il suffit. Port 0 non supporté (la validation le refuse déjà).
  - Le tunnel se connecte d'abord à la cible locale, puis accepte le canal ; échec → rejet `ConnectFailed` + santé `Unreachable`. Sonde active = connexion TCP locale. Refus du `tcpip_forward` → `Failed { RemoteForwardRefused }` sans retry ; autre erreur de requête = perte de session (backoff). Arrêt/redémarrage/changement → `cancel_tcpip_forward` (5 s max, échec loggé) ; désinscription garantie par un garde sur tous les chemins.
  - `CountingStream` : constructeurs `on_client_side`/`on_target_side` pour que `bytes_in`/`bytes_out` gardent leur sens en remote.
- [x] 9.2 Dynamic : serveur SOCKS5 (`fast-socks5`) → `direct-tcpip`
  - `fast-socks5` 1.0 (API typestate `Socks5ServerProtocol`, ajoute seulement `socket2` au lockfile). Sans authentification, CONNECT seul (BIND/UDP → `CommandNotSupported`), noms de domaine transmis non résolus au serveur (comme `ssh -D`), handshake borné à 10 s, erreurs de canal traduites en réponses SOCKS (refus, interdit, échec général). Santé toujours `Unknown` (pas de cible fixe, pas de sonde).
  - Local, dynamic et remote partagent la même boucle (`ForwardRun` + `InboundSource` écouteur/canaux transférés), le relais et les métriques.
  - Validation : un tunnel dynamique avec `target_host`/`target_port` est refusé (`DynamicTunnelWithTarget`). `FailureReason::UnsupportedKind` supprimé (moteur, DTO, CLI, UI ; protocole v2 non publié, pas de nouvelle version).
- [x] 9.3 UI : sélecteur de type, champs adaptés
  - Groupe radio « Type de tunnel » (`RadioOptions` de la lib, descriptions reliées) : Local (-L), Distant (-R), Dynamique (-D, SOCKS). Remote : « Port d'écoute sur le serveur », « Hôte cible (depuis ce poste) », indice de bind = `defaultRemoteBindAddress`. Dynamic : champs cible masqués et jamais envoyés (valeurs conservées si on revient en arrière).
  - Libellés : `nova:8080 → 127.0.0.1:3000` (« port 8080 du serveur nova vers 127.0.0.1:3000 »), `1080 (SOCKS via nova)` (« proxy SOCKS sur le port 1080 via nova »). Import des externes ouvert aux forwards remote/dynamic. CLI : `3333 → SOCKS via nova`, `0.0.0.0:3333 on nova → …` pour un bind remote non par défaut.
- [x] 9.4 Tests d'intégration
  - Serveur de test : `tcpip_forward` réel sur 127.0.0.1 (port 0 alloué, ré-essai ~200 ms pour re-binder après coupure), `cancel_tcpip_forward`, option `refusing_remote_forwards()`. 31 tests runtime : SOCKS par IP et par nom, cible refusée (réponse SOCKS, tunnel reste Active), remote aller-retour, refus, arrêt qui annule le forward (vérifié par mutation), reconnexion puis forward rétabli, local + remote sur une session, cible locale down → `Unreachable`.
  - Un échec isolé non reproduit (`should_keep_the_local_tunnel_forwarding_when_a_dynamic_tunnel_on_the_same_host_is_toggled`, premier run complet ; cause probable : collision de `free_port()`), puis 3 runs complets + ~20 runs runtime + 120 runs parallèles du test verts.
  - **Non fait** : `curl --socks5` contre un vrai sshd (l'authentification vers le sshd local attend une confirmation de l'agent, cf. 11.7) ; pas d'E2E CLI pour remote/dynamic.

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows) et tests verts (1706 tests Rust ×3, 654 tests front).

## Phase 10 — X11 (reste : validation manuelle 10.6)
- [x] 10.1 Parser `Xauthority` local + résolution du display (TCP / socket Unix)
  - Display local : `hosts.<alias>.x11_display` > `settings.x11_display` > `$DISPLAY` > `127.0.0.1:0.0` sous Windows > erreur. Syntaxe `[host[/unix]]:N[.S]`, `unix:N`, IPv6 entre crochets ; host vide/`unix` → `/tmp/.X11-unix/XN` (Unix), sinon TCP `6000+N`. `host/unix:N` traité comme `unix:N` ; le chemin launchd de macOS n'est pas supporté.
  - Parser binaire maison, tolérant (entrée finale tronquée ignorée avec avertissement, fichier absent = pas de cookie → auth vidée, cas VcXsrv « Disable access control »). Recherche `MIT-MAGIC-COOKIE-1` par numéro de display : Local (nom d'hôte) ou Wild pour un display local, Internet/Internet6 pour TCP (+ Local pour `localhost`/`127.0.0.1`). Wild correspond à tout display, comme le serveur X.
  - Environnement (`DISPLAY`, `XAUTHORITY`, home, nom d'hôte, plateforme) derrière le trait `X11Environment` (`EngineDependencies.x11_environment`) ; `whoami` ajouté pour le nom d'hôte.
- [x] 10.2 Ancre X11 par host (`request_x11` cookie factice, lecture du `DISPLAY` distant)
  - Un acteur par host (comme les tunnels) : bail de session, `request_x11` (cookie factice 16 octets aléatoires, multi-connexions), `echo "$DISPLAY"; exec sleep 2147483647`, première ligne → `Active { display }`. Refus → `Failed`, `DISPLAY` vide → `Failed` (« X11Forwarding activé sur le serveur ? »), perte de session/canal → `Reconnecting` (mêmes backoff/classification que les tunnels). Redémarrable depuis `Failed`.
  - **Écart PLAN** : quitter avec des ancres actives demande confirmation comme pour les tunnels (le compteur `tunnels_running` inclut les sessions X11 ; libellés CLI/UI adaptés : « tunnels ou sessions X11 »).
- [x] 10.3 Canaux entrants : substitution du cookie dans le setup X11 (ou auth vide)
  - `X11Router` par session : **tout canal X11 est refusé tant que X11 n'a pas été activé** sur la session (le comportement par défaut de russh acceptait). Le display et le cookie réel sont figés à la première activation : un changement de `x11_display` ne vaut que pour les nouvelles sessions.
  - Display local contacté avant d'accepter le canal (sinon `ConnectFailed`) ; préfixe de setup vérifié (nom `MIT-MAGIC-COOKIE-1`, cookie comparé en temps constant, champs ≤ 256 octets, 10 s max) puis réécrit avec le vrai cookie ou une auth vide ; sinon canal fermé, rien n'atteint le serveur X.
- [x] 10.4 Lanceurs X11 par host
  - `x11.launch { host, name }` : lanceur pris dans `[[x11_launchers]]`, session X11 dédiée, réponse dès l'`exec` accepté ; sorties journalisées (stdout info, stderr warn, champs `host`/`launcher`) via `tracing` seulement, pas dans les logs de tunnel. Les lanceurs s'éditent dans `tunnels.toml` (pas de CRUD UI).
- [x] 10.5 UI : display distant + copier `export DISPLAY`, lanceurs, avertissement sécurité
  - Bouton « X11 pour nova » dans l'en-tête de chaque groupe d'hôte + statut texte/icône (« X11 actif : localhost:10.0 »). Drawer « X11 : nova » : note de sécurité toujours visible (`role="note"`), état (badge icône + texte), *Démarrer/Arrêter X11* avec confirmation `alertdialog` au premier démarrage par hôte et par session UI, « Copier export DISPLAY=… », lanceurs avec *Lancer*, extrait `[[x11_launchers]]` si aucun. Annonces live polies. Champ « Display X11 local » dans les réglages.
  - Limite : un hôte sans tunnel n'a pas de groupe, donc pas d'accès X11 depuis l'UI.
  - Protocole : `x11.start`/`stop`/`launch`, `x11.changed`, `SnapshotDto.x11Anchors`/`x11Launchers` (additif, protocole v2 non publié). CLI `status` : bloc « X11: » ; `UnknownLauncher` → `not_found`, problèmes de display/refus → `invalid_state`.
  - Tests : 28 tests d'intégration X11 (ancre, refus, DISPLAY vide, arrêt, reconnexion, canal relayé vers un faux serveur X avec cookie remplacé, mauvais cookie, sans Xauthority, canal non sollicité refusé, lanceurs), ~70 unitaires, ~45 front.
- [ ] 10.6 Validation manuelle : VcXsrv (Windows) + Xorg/Xwayland (Linux)
  - À faire par un humain : aucun sshd réel utilisable ici (l'authentification vers le sshd local attend une confirmation de l'agent). Socket Unix testé en connexion seulement ; rien d'exécuté sous Windows.

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows) et tests verts (1845 tests Rust ×2, 716 tests front).

## Phase 11 — Confort ✔ (rendu réel dans une webview non vérifié)
- [x] 11.1 Import des `LocalForward`/`RemoteForward`/`DynamicForward` de `~/.ssh/config`
  - `engine::import::preview_ssh_config` : forwards résolus de chaque alias concret (`list_hosts`), forwards de `Host *` donc répétés par alias (signalés en doublon par l'UI). Socket Unix et `RemoteForward` sans cible (SOCKS distant) → avertissement `unsupportedForward` ; alias illisible → `hostResolutionFailed`.
- [x] 11.2 Import par collage (script PS, liste de ports, specs `-L a:b:c`)
  - `engine::import::preview_script`, parser maison sans dépendance : commentaires PS `<# … #>` et `#`, continuations (backtick, `\`), tableaux `@( … )` (virgules optionnelles) / `name=( … )` / lignes de ports, specs `-L/-R/-D` partout (y compris dans des chaînes), modèles à variable (`-L ${_}:127.0.0.1:${_}`) appliqués à la liste de ports, lignes `a:b:c` nues. Le commentaire d'un port devient le nom proposé. Toutes les specs passent par les parsers `ssh-config` (avertissement `malformedForward` si illisible) ; `parse_ssh_command_line` ne sert qu'à la destination (variables/splats `@forwards` ignorés).
  - Destination → alias : alias identique, sinon premier alias dont `hostname` (+ `user`/`port` si donnés, `-l`/`-p` compris) correspond ; sinon `unknownDestination` et l'UI demande l'hôte. `SshInvocation.destination` devient `SshDestination { user, host, port }`. Script PowerShell d'origine en test (8 tunnels locaux nommés d'après les commentaires).
  - Sur ce poste `nova` n'a pas de `HostName` (résolu par DNS) : le script d'origine (`jules@198.51.100.157`) y donne l'avertissement « aucun alias » et l'hôte se choisit dans l'UI.
  - Protocole (additif, version inchangée) : `import.ssh_config`, `import.script { text }` (≤ 256 Kio, sinon `invalid_request`) → `ImportPreviewDto { candidates, warnings }`. L'aperçu ne crée rien.
  - UI : tiroir « Importer des tunnels » (bouton d'en-tête, raccourci `i`, palette), onglets « Coller un script » / « Depuis ~/.ssh/config », aperçu avec cases à cocher, noms proposés uniques, conflits détectés côté UI (déjà configuré, port déjà utilisé, doublon dans l'import → décochés par défaut), hôte par défaut pour les candidats sans destination, tags communs. Création séquentielle par `tunnel.create` (pas de méthode groupée) : les échecs restent listés dans le tiroir, le daemon reste l'arbitre.
  - Non fait : `$env:DISPLAY` du script n'est pas repris en `hosts.<alias>.x11_display` ; pas de sous-commande CLI d'import.
- [x] 11.3 Palette de commandes `Ctrl+K`
  - Construite dans l'app (le `Combobox` de la lib est un select) : `Modal` de la lib + motif combobox/listbox (`aria-activedescendant`), filtre insensible à la casse et aux accents (tous les mots), 50 résultats max, compteur annoncé. Commandes générales, par tunnel (démarrer/arrêter, détails, modifier, navigateur), par tag, par hôte (config résolue, X11). `Ctrl+K`/`Meta+K` même depuis un champ, jamais par-dessus une modale ; bouton d'en-tête avec `aria-keyshortcuts`.
- [x] 11.4 Panneau logs live + métriques dans le drawer
  - Nouveau tiroir « Tunnel <nom> » (entrée « Détails » du menu de ligne, palette) : état + santé, métriques live, actions, journal (`tunnel.logs` puis événements `tunnel.log`, 500 entrées max, `role="log"` en `aria-live="off"` car les changements d'état sont déjà annoncés, messages moteur en `lang="en"`, défilement collé au bas sauf si l'utilisateur remonte). L'édition reste un tiroir séparé.
  - Le moteur journalise désormais l'échec définitif d'une connexion (« cannot connect: … »).
- [x] 11.5 Ouvrir dans le navigateur, copier la commande `ssh` équivalente
  - **Écart PLAN §7** : pas de clic sur le port dans la ligne (évite un arrêt de tabulation de plus dans la liste à tabindex itinérant) ; « Ouvrir dans le navigateur » est dans le menu de ligne, le tiroir de détails et la palette (tunnels locaux, actifs).
  - Commande Tauri `open_in_browser { host, port }` : URL construite côté natif, hôte limité à `localhost` ou une IP littérale (`invalid_target` sinon), `tauri-plugin-opener` utilisé depuis Rust seulement (aucune permission d'ouverture d'URL donnée à la webview). Adresse d'écoute joker → `localhost`.
  - Commande copiée : `ssh -N -L [addr:]port:cible:port hôte` (`-R`/`-D` selon le type), adresse de bind seulement si le tunnel en fixe une.
- [x] 11.6 Vue « config résolue » d'un host (équivalent `ssh -G`)
  - Rendu `ssh -G` sorti des tests différentiels vers `ssh-config` (`ResolvedConfig::ssh_g_options`, directives non modélisées en fin de liste) ; le test différentiel l'utilise. `HostConfigResolver::resolve_with_warnings` (implémentation par défaut sans avertissement).
  - Protocole : `hosts.resolve { host }` → `{ host, options: [{ key, value }], warnings }` (avertissements en anglais, affichés en `lang="en"`).
  - UI : bouton « Configuration de <hôte> » dans l'en-tête de chaque groupe + palette (tous les alias, y compris ceux sans tunnel) ; tableau filtrable, compteur annoncé, copie, avertissements.
- [x] 11.7 Authentification bloquée : délai sur les étapes non humaines (connexion, signature par l'agent SSH), jamais sur une saisie de passphrase/mot de passe ; échec explicite (« l'agent SSH ne répond pas ») au lieu d'un « Démarrage… » sans fin ; arrêt possible pendant le démarrage. Vu en 6.14 avec un agent transféré qui attend une confirmation.
  - `ConnectTimeouts { connect, authentication_step, agent_response }`, 30 s chacun (constantes, injectables pour les tests, pas de réglage utilisateur). `connect` sert de `ConnectTimeout` quand l'hôte n'en fixe pas (TCP, handshake, ProxyCommand, sauts).
  - `bounded` + `DeadlinePause` (généralise le mécanisme du handshake) : l'horloge s'arrête pendant toute question à l'utilisateur (`PausingPrompter`) et repart pleine ensuite. Authentification sans réponse du serveur → `AuthenticationTimeout`, relancée (réseau).
  - Agent : connexion, `request_identities` et chaque signature (`BoundedAgentSigner`, seule la réponse de l'agent est bornée) → `AgentUnresponsive`, **jamais relancé** (une relance redemanderait une confirmation). Nouveau `FailureReason::AgentUnresponsive` / DTO `{ kind: "agentUnresponsive", timeoutMs }` ; UI « L'agent SSH ne répond pas (aucune réponse en 30 s) : attend-il une confirmation ? », CLI idem en anglais. Les étapes de l'agent suspendent aussi l'horloge d'authentification (sinon le délai d'authentification, relancé, partirait avant celui de l'agent).
  - Arrêt pendant `Starting` : déjà possible côté tunnel ; en plus, la tentative de connexion partagée du `SessionManager` est abandonnée quand plus personne ne l'attend (tunnels et ancres X11), et une invite en attente est retirée (`prompt.resolved`) quand sa tâche est abandonnée (garde au drop dans le daemon).
  - 8 tests d'intégration (agent muet, serveur muet, mot de passe saisi après le délai, arrêt pendant le démarrage, nouvelle tentative), 5 runs consécutifs verts.

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows) et tests verts (1972 tests Rust, 926 tests front). Rien d'exécuté dans une vraie webview (tiroirs, palette, ouverture du navigateur), ni sous Windows.

## Phase 12 — Démarrage & secrets ✔ (code Windows jamais exécuté, rendu réel non vérifié)
- [x] 12.1 Lancement au login (registre `Run` Windows, XDG autostart Linux)
  - `daemon/login_launch/` (trait `LoginLaunch`) : commande = exécutable courant + `daemon` (daemon + tray, sans fenêtre). Linux : `$XDG_CONFIG_HOME/autostart/tunneler.desktop` écrit atomiquement, `Exec` cité selon la spec, marqueur `X-Tunneler-Autostart=true` pour ne jamais toucher un fichier étranger. Windows : valeur `Tunneler` sous `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (`winreg`). macOS : non supporté, avertissement unique.
  - Appliqué au démarrage du daemon (rafraîchit aussi un chemin d'exécutable périmé) et à chaque `config.reloaded` (`settings.update` ou édition à la main) ; échec loggé, réessayé au changement suivant. Pas de retour d'erreur dans l'UI.
- [x] 12.2 `start_on_launch_tags` + `restore_on_launch` (`state.toml`)
  - `state.toml` (`BaseDirs::state_dir()` sous Linux, dossier de données local ailleurs, `tunneler/state.toml` ; option `--state` / `TUNNELER_STATE` comme `--config`) : `version = 1`, `running` (tunnels voulus actifs), `keyring_entries` (12.3). Écriture atomique hors executor, toujours l'état le plus récent, échec loggé sans faire échouer la commande ; fichier illisible → état vide + avertissement ; vidé (flush) à l'arrêt.
  - Intention enregistrée dans l'engine (UI, CLI et tray passent par lui) : `start_tunnel`/`start_tag`/`adopt_external` ajoutent, `stop_tunnel`/`stop_tag`/`delete_tunnel` retirent, `restart` et `shutdown` (« Tout arrêter et quitter », signaux) ne changent rien. Un tunnel en échec ou externe reste voulu.
  - Au lancement (`Engine::start_launch_tunnels`, une fois le daemon à l'écoute) : union de `running` (si `restore_on_launch`) et des tunnels portant un tag de `start_on_launch_tags`, ordre de la config, sans doublon. Ces démarrages ne sont pas enregistrés comme intention (un tunnel lancé seulement par tag n'entre pas dans `running`). Ancres X11 non restaurées.
  - UI : section « Démarrage » des paramètres (lancement à l'ouverture de session, rétablir les tunnels, tags démarrés au lancement).
- [x] 12.3 Cache de passphrase en mémoire (`zeroize`) + trousseau OS optionnel (`keyring`)
  - `PassphraseVault` (engine) : cache mémoire par chemin canonique de clé (`Zeroizing`, durée de vie du daemon), puis trousseau si `settings.use_os_keyring` (nouveau réglage, défaut `false`). Passphrase mémorisée refusée → évincée (mémoire + trousseau) puis invite. Appels au trousseau hors executor sous `DeadlinePause` (un déverrouillage du trousseau est une étape humaine) ; erreur du trousseau loggée = non trouvée. Sous `BatchMode`, les passphrases mémorisées ne sont pas essayées non plus (garde existante).
  - Prompt : `CredentialPrompter::passphrase` rend `PassphraseAnswer { passphrase, retention: Never | Session | Keyring }`. Protocole : `credential.respond` accepte `remember: "session" | "keyring"` (absent = pas mémorisé), `secrets.forget` → `{ forgottenCount }` (vide le cache et supprime les entrées listées dans `state.toml`). Mots de passe et keyboard-interactive jamais mémorisés.
  - Trousseau : `keyring` 4 (fonctionnalités par défaut = stores natifs : Credential Manager Windows, Secret Service via zbus en Rust pur sous Linux, Keychain macOS), service `tunneler`, compte `passphrase:<chemin>`. Sans Secret Service, avertissement à chaque appel et mémoire seule.
  - UI : choix « Mémoriser la passphrase » dans l'invite (Ne pas mémoriser / Jusqu'à la fermeture de Tunneler (défaut) / Trousseau, ce dernier seulement si le réglage est actif), section « Secrets » des paramètres (réglage trousseau + « Oublier les passphrases mémorisées » avec confirmation). La CLI répond sans `remember` (jamais mémorisé) ; pas de sous-commande CLI pour l'oubli.
  - Entrée pour valider une invite de passphrase non testée (jsdom ne soumet pas un formulaire dont le bouton est hors du `<form>`) : à vérifier à la main.

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows) et tests verts (2059 tests Rust, 949 tests front). Code Windows (`winreg`, Credential Manager) compilé mais jamais exécuté ; macOS jamais compilé.

## Phase 13 — Packaging & release (Windows jamais exécuté ; audit a11y à faire à la main)
- [x] 13.1 Windows : zip portable (`tunneler.exe` + `tunneler-ui.exe`), icônes, métadonnées
  - `tunneler-<version>-windows-x86_64.zip` : les deux exe côte à côte + `README.md`. Non signés (SmartScreen documenté).
  - `tunneler.exe` reste un programme console (c'est aussi la CLI). Lancé hors terminal (double-clic, entrée `Run` du login), Windows lui donne une console qui resterait ouverte toute la vie du daemon : le daemon s'en détache (`FreeConsole`) quand il en est le seul utilisateur (`GetConsoleProcessList` = 1), avant l'init des logs (→ logs en fichiers seulement). Depuis un terminal, la console est gardée. La fenêtre clignote brièvement au login. `windows-sys`, seul `unsafe` du workspace (module `daemon/console.rs`, `#[allow(unsafe_code)]` + `// SAFETY:`).
  - Icône + VERSIONINFO de `tunneler.exe` via `build.rs` (`winresource`), uniquement en cible MSVC (le cross-clippy gnu local n'a pas de `windres`). `tunneler-ui.exe` : ressources par `tauri-build` depuis `tauri.conf.json` (publisher, copyright, descriptions ajoutés).
  - **Piège** : `cargo build --release -p tunneler-ui` sans `tauri/custom-protocol` compile en `cfg(dev)` et charge le `devUrl`. L'UI de release se construit uniquement par `pnpm tauri build` (`--no-bundle` sous Windows). `pnpm tauri build` réécrit `app/src-tauri/Cargo.toml` (`features = []`) : ne pas committer cette réécriture.
- [x] 13.2 Linux : tar.gz (daemon) + AppImage (UI), dépendances documentées (webkit2gtk-4.1)
  - `tunneler-<version>-linux-x86_64.tar.gz` contient `tunneler` **et** `tunneler-ui` (binaire simple, demande `libwebkit2gtk-4.1-0` + `libgtk-3-0`) : prêt à l'emploi, le daemon trouve l'UI à côté de lui. L'AppImage (`tunneler-ui-<version>-x86_64.AppImage`, ~108 Mio) est l'alternative quand WebKitGTK 4.1 n'est pas installable : à renommer `tunneler-ui` à côté de `tunneler` ou via `TUNNELER_UI_BINARY`. Le daemon n'est pas dans l'AppImage (son chemin de montage temporaire casserait le lancement au login).
  - Daemon : libc seule. Construit sur Ubuntu 22.04 (glibc ≥ 2.35). Pas d'appindicator/libsecret/xdo liés : tray et trousseau passent par D-Bus.
  - Tailles locales : `tunneler` 22 Mio, `tunneler-ui` 17 Mio (non strippés, `[profile.release]` par défaut).
- [x] 13.3 Mesures RAM (daemon idle, daemon 10 tunnels, UI ouverte) documentées dans le README
  - Linux (release 0.1.0, Debian 13, tray actif, sshd OpenSSH isolé en local) : daemon au repos **16,5 Mio** RSS, 10 tunnels (une session partagée, un peu de trafic) **19,4 Mio** — sous la cible D2 de 25 Mo. UI ouverte : **~410 Mio PSS** (`tunneler-ui` 110 + `WebKitWebProcess` 257 + `WebKitNetworkProcess` 43 ; 600 Mio en somme de RSS), sous Xvfb en rendu logiciel (llvmpipe), donc probablement surestimé ; au-dessus des 100-200 Mo envisagés par D5 — payé seulement fenêtre ouverte. À remesurer sur un vrai bureau.
  - Windows (mesuré par l'utilisateur, release 0.1.0) : daemon **25,2 Mio** de working set (9,6 Mio privés ; nombre de tunnels actifs non relevé, 10 tunnels non mesuré). UI ouverte : **227 Mio privés à 428 Mio** de working set cumulé (`tunneler-ui` 5,6 Mio privés + 6 processus `msedgewebview2`), même ordre que Linux.
- [x] 13.4 Workflow de release CI
  - `.github/workflows/release.yml` : tag `v*` ou `workflow_dispatch`. Job `version` (version du workspace = `tauri.conf.json` = `app/package.json`, et = tag), jobs `windows` (windows-latest) et `linux` (ubuntu-22.04) qui publient les paquets en artifacts, job `release` (tags seulement) : `SHA256SUMS` + `gh release create --draft --verify-tag --generate-notes`. Pas de macOS (plan : sans régression seulement). Procédure dans le README (« Releasing »).
- [~] 13.5 Audit accessibilité final (WCAG 2.2 AA, checklist PLAN §8) sur Windows et Linux, résultats documentés
  - Grille dans [`AUDIT_A11Y.md`](./AUDIT_A11Y.md), préremplie avec les acquis (6.14, phase 7) ; reste à faire à la main sur les binaires de release (NVDA, Orca, contraste élevé, zoom 200 %).

- CI enfin verte sur les 3 OS (premier passage depuis la phase 9) après corrections de tests uniquement : macOS ne compilait pas (`mode_t` en `u16`, code mort du tray sans tray) ; sockets de test créées dans des dossiers temporaires non privés (refusé sous macOS, où le `chmod` de la socket n'est pas atomique : dossiers de test en 0700) ; nom de processus `ssh.exe` sous Windows ; fixtures en CRLF sous Windows (`.gitattributes` `eol=lf`) ; test différentiel en conteneur (`HOME` ≠ home passwd utilisé par `ssh` pour `~`). `cargo test --no-fail-fast` en CI.
- **Lacunes connues** : macOS ne fournit pas le pid du pair IPC (`getpeereid`) → `peer.process_id` absent (informatif, test ignoré sous macOS). Le résolveur `ssh_config` développe `~` avec `$HOME`, OpenSSH avec le home de passwd (identiques hors conteneurs). Un `tunnels.toml` en CRLF réécrit après modification n'est pas garanti identique hors des lignes touchées (non testé).

**Vérifications** : fmt, clippy (workspace, `tunneler-ui`, cross-clippy Windows), tests (2062 Rust), `pnpm check`/`build`, `actionlint` verts. `pnpm tauri build --bundles appimage` et `--no-bundle` OK en local. Rien exécuté sous Windows ni sur MSVC (ressources, `FreeConsole`).
