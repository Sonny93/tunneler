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
  - Notés : le bouton du menu d'actions est exposé « combo box » (mapping WebKitGTK des boutons `aria-haspopup`) ; une authentification qui ne répond jamais (agent SSH qui attend une confirmation) laisse le tunnel en « Démarrage… » sans fin — pas de délai d'authentification côté moteur (suivi moteur, hors phase 6).
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
