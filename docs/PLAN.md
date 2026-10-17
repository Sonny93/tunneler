# Tunneler — Plan

Gestionnaire de tunnels SSH portable (Windows + Linux, macOS sans régression), léger, avec UI moderne.
Remplace le script PowerShell `ssh -N -L ...` : activation/désactivation des tunnels à la volée, groupes via tags, détection des tunnels lancés hors de l'app.

La liste des tâches à cocher vit dans [`TASKS.md`](./TASKS.md). Ce fichier décrit le *quoi* et le *pourquoi* ; il est mis à jour quand une décision change.

---

## 1. Décisions verrouillées

| # | Décision | Raison |
|---|---|---|
| D1 | **Client SSH embarqué (`russh`)**, pas de binaire `ssh` système | Une session par host, tunnels = canaux : toggle instantané sans reconnexion, mêmes comportements sur tous les OS, contrôle total (états, métriques, prompts, X11 sans `xauth`) |
| D2 | **Architecture daemon + UI à la demande** | Le daemon headless (tokio + russh + tray) reste léger (~cible < 25 Mo RSS). L'UI Tauri est un process séparé qui ne vit que quand la fenêtre est ouverte → 0 Mo quand fermée |
| D3 | **Parser `ssh_config` maison et complet** | `ssh2-config` 0.8 n'a ni `Match` ni expansion de tokens, `russh-config` est un jouet. Comportement de référence : OpenSSH (`ssh -G`) |
| D4 | **Parser `known_hosts` maison** | `russh::keys::known_hosts` : pas de wildcards/négation, pas de `@cert-authority`/`@revoked`, chemin figé, écrit en clair |
| D5 | **UI : Tauri 2 + React 19 + TypeScript + Vite + UnoCSS + `@minimalstuff/ui`** | Rendu soigné, stack maîtrisée, lib de composants maison (UnoCSS, React 19, base ARIA correcte). Le coût RAM du webview (~100-200 Mo) n'est payé que fenêtre ouverte (D2) |
| D12 | **Outillage front : oxlint + oxfmt via `@minimalstuff/tooling`** (+ plugin `jsx-a11y` d'oxlint, absent du preset React) | Presets partagés, rapide, cohérent avec les autres projets. Rust : `rustfmt` + `clippy` |
| D6 | **Tray : `tray-icon` avec feature `ksni` sur Linux** | Win32 natif sous Windows (boucle `winit`), StatusNotifierItem D-Bus pur Rust sous Linux (pas de GTK dans le daemon). Pas de tray sous macOS (la boucle exigerait le thread principal) |
| D7 | **IPC : `interprocess` (local socket / named pipe), JSON lines** | Cross-platform, restreint à l'utilisateur courant. Même protocole pour l'UI et la CLI |
| D8 | **Persistance TOML lisible à la main** dans le dossier de config utilisateur | Éditable, diffable, rechargé à chaud |
| D9 | **Tags (n-n) plutôt que groupes (1-n)** | Un tunnel peut appartenir à plusieurs ensembles (`dev`, `adonis`, `mail`...) ; toggle d'un tag = toggle de ses tunnels |
| D10 | **Single instance portée par le daemon** (bind du socket IPC = verrou) | Pas besoin de `tauri-plugin-single-instance` : une 2ᵉ invocation devient un client qui demande au daemon d'afficher l'UI |
| D11 | **Accessibilité = exigence, pas une finition** : UI conforme WCAG 2.2 AA, CLI utilisable sans couleur ni TTY (voir §8) | Une fonctionnalité n'est pas « faite » tant qu'elle n'est pas utilisable au clavier seul, au lecteur d'écran et sans dépendre de la couleur |

### Conséquence de D1/D2 sur la fermeture
Les tunnels vivent *dans* le daemon : ils ne peuvent pas survivre à sa fermeture. L'option « quitter en laissant tourner » proposée initialement disparaît.
- **Fermer la fenêtre** : l'UI se termine, le daemon et les tunnels continuent (comportement normal, pas de confirmation).
- **Quitter Tunneler** (tray ou UI) avec des tunnels actifs → confirmation : *Tout arrêter et quitter* / *Annuler*.

---

## 2. Architecture

```
┌──────────────────────── tunneler (daemon, toujours actif) ────────────────────────┐
│  tray (tray-icon/ksni)      IPC server (interprocess)       store (TOML + notify) │
│          │                          │                               │             │
│          └──────────── engine ──────┴───────────────────────────────┘             │
│      SessionManager (1 session russh / host, refcount, keepalive, reconnect)      │
│      Forwards : local · remote · dynamic (SOCKS5) · X11                           │
│      Auth (agent → clés → password/kbd-interactive via prompts)                   │
│      HostKey (known-hosts) · PortScanner (tunnels externes) · Health probe        │
└───────────────────────────────────────▲───────────────────────────────────────────┘
                                        │ JSON lines (local socket / named pipe)
                 ┌──────────────────────┴───────────────────────┐
     tunneler-ui (Tauri 2, à la demande)              tunneler <cmd> (CLI)
```

### Workspace Cargo

| Crate | Type | Rôle |
|---|---|---|
| `crates/ssh-config` | lib | Lexer + parser + résolution `ssh_config` (`Host`, `Match`, `Include`, tokens, first-match-wins, directives multi-valeurs). Aucune dépendance russh |
| `crates/known-hosts` | lib | Lecture/écriture `known_hosts` (hashé `\|1\|`, wildcards, négation, `[host]:port`, `@cert-authority`, `@revoked`, `UserKnownHostsFile`/`GlobalKnownHostsFile`, `HashKnownHosts`) |
| `crates/engine` | lib | Cœur métier : sessions, auth, forwards, états, métriques, santé, scan de ports, store. Dépend d'abstractions (`CredentialPrompter`, `HostKeyVerifier`, `Clock`...) |
| `crates/ipc` | lib | Types du protocole (serde, versionnés) + client + serveur |
| `crates/tunneler` | bin `tunneler` | Daemon (`tunneler daemon`, défaut) + CLI (`up`, `down`, `status`, `list`, `open`, `quit`) + tray |
| `app/` | bin `tunneler-ui` | Tauri 2 : `src-tauri` = pont IPC minimal, `src` = React |

`app/src-tauri` est membre du workspace mais hors `default-members` : les crates du daemon se construisent sans les dépendances système du webview ; le crate UI se construit explicitement (`-p tunneler-ui`).

### Démarrage / single instance
1. `tunneler` (sans argument) tente de binder le socket IPC.
2. Succès → devient le daemon et ouvre l'UI (`tunneler daemon` : daemon seul). Échec car un daemon répond → client : envoie `ui.show` et quitte.
3. Socket orphelin (daemon crashé) → reclaim (`reclaim_name`).
4. `tunneler-ui` lancé seul sans daemon → spawn `tunneler daemon` détaché, puis se connecte.

Nom du socket : `$XDG_RUNTIME_DIR/tunneler.sock` (mode `0600`) sous Linux, `\\.\pipe\tunneler-<user>` avec security descriptor restreint à l'utilisateur sous Windows.

---

## 3. Moteur (engine)

### Sessions
- Une session russh par **alias de host** résolu via `ssh-config`. Ouverte à la demande au premier tunnel démarré, fermée après le dernier + délai de grâce (`session_idle_grace_s`).
- Transport : TCP direct, `ProxyJump` (chaîne : session parente → `channel_open_direct_tcpip` → `into_stream` → `connect_stream`), `ProxyCommand` (stdio du process enfant fusionné en un flux duplex).
- Keepalive : `keepalive_interval` / `keepalive_max` (depuis `ServerAliveInterval`/`ServerAliveCountMax`, sinon settings).
- Perte de session → tous ses tunnels passent `Reconnecting`, backoff exponentiel avec jitter, plafonné.

### Authentification
Ordre conforme à OpenSSH, filtré par `PreferredAuthentications` / `IdentitiesOnly` :
1. Agent : `SSH_AUTH_SOCK` / `IdentityAgent` (Unix), named pipe OpenSSH puis Pageant (Windows).
2. `IdentityFile` (OpenSSH, PKCS#8, PuTTY `.ppk`) ; clé chiffrée → demande de passphrase.
3. `keyboard-interactive`, puis `password`.

Les prompts passent par le trait `CredentialPrompter` → événement IPC `credential.request`. Si aucune UI n'est connectée, le daemon la lance. Secrets en mémoire `zeroize`. Passphrase de clé mémorisable au choix dans l'invite : pas du tout, jusqu'à l'arrêt du daemon, ou dans le trousseau OS (`keyring`, si `use_os_keyring`) ; une passphrase mémorisée refusée est oubliée puis redemandée. Mots de passe jamais mémorisés.

### Vérification de la clé d'hôte
- Connue et identique → OK. Inconnue → prompt UI (empreinte SHA256, type) : *Accepter et enregistrer* / *Accepter une fois* / *Refuser*. Respecte `StrictHostKeyChecking`.
- **Changée ou révoquée → échec dur**, alerte explicite, jamais de contournement en un clic.

### Types de tunnels

| Kind | Écoute | Cible | Implémentation |
|---|---|---|---|
| `local` (`-L`) | `listen_address:listen_port` local | `target_host:target_port` vu depuis le serveur | `TcpListener` local → `channel_open_direct_tcpip` par connexion |
| `remote` (`-R`) | `listen_address:listen_port` sur le serveur | `target_host:target_port` local | `tcpip_forward` + callback `server_channel_open_forwarded_tcpip` |
| `dynamic` (`-D`) | `listen_address:listen_port` local | — | SOCKS5 (`fast-socks5`) → `direct-tcpip` |
| X11 | — | display X local | voir §5 |

Règles par défaut : `target_port` absent = `listen_port` ; `listen_address` absent = `settings.default_bind_address` (local) ou `settings.default_remote_bind_address` (remote) ; `target_host` absent = `127.0.0.1`.

### États d'un tunnel
`Stopped` · `Starting` · `Active` · `Reconnecting { attempt, next_retry_at }` · `Failed { reason }` · `External { pid, process }` · `Conflict { pid, process }`

Santé de la cible, indépendante de l'état : `Unknown` · `Reachable` · `Unreachable { since, error }`.
- Passif : un `direct-tcpip` refusé (`ConnectFailed`) marque la cible injoignable (« tunnel OK, service distant down »).
- Actif (option `health_probe`) : ouverture/fermeture périodique d'un canal vers la cible, uniquement quand une UI est connectée.

Métriques par tunnel : connexions actives, octets in/out, dernière activité. Logs : ring buffer par tunnel + fichier de log tournant (`tracing`).

### Tunnels externes
- `PortScanner` (`listeners` + `sysinfo`) : process propriétaire du port d'écoute de chaque tunnel configuré, au démarrage d'un tunnel, à l'ouverture de l'UI, puis toutes les N secondes tant qu'une UI est connectée.
- Port tenu par `ssh`/`ssh.exe` dont la ligne de commande contient un `-L`/`-R`/`-D` correspondant (type, port d'écoute, adresse compatible, cible équivalente ; la destination ssh n'est pas comparée) → `External` : le tunnel est **considéré comme fourni** (réutilisé). Actions : *Adopter* (kill + relance gérée de tous les tunnels fournis par ce processus), *Kill*, *Ignorer* (le daemon cesse de signaler ce processus jusqu'à son arrêt, non persisté).
- Port tenu par autre chose → `Conflict` (ex. « port occupé par node.exe (pid 4242) »). Action : *Kill* (avec confirmation).
- Tunnels `ssh` détectés hors config → section « Externes non gérés » avec *Importer comme tunnel* (formulaire de création prérempli → `tunnel.create`), *Kill*, *Ignorer*.
- Processus identifié par pid + heure de démarrage : un pid réutilisé n'est jamais tué. Le daemon s'exclut du scan.
- Limite connue : sous Linux, les sockets d'un autre utilisateur ne donnent pas de PID sans root → affiché « process inconnu ».

---

## 4. Persistance

Fichier `tunnels.toml` dans le dossier de config (`directories` : `~/.config/tunneler/`, `%APPDATA%\tunneler\`). Écriture atomique (tmp + rename), rechargement à chaud (`notify`), schéma versionné avec migrations.

```toml
version = 1

[settings]
default_bind_address = "127.0.0.1"
default_remote_bind_address = "localhost"
keepalive_interval_s = 15
keepalive_max = 3
session_idle_grace_s = 30
reconnect_initial_delay_ms = 1000
reconnect_max_delay_ms = 60000
health_probe = true
restore_on_launch = true          # relance les tunnels actifs à la fermeture précédente
start_on_launch_tags = []         # tags démarrés au lancement du daemon
launch_at_login = false
use_os_keyring = false            # passphrases mémorisables dans le trousseau du système
x11_display = ""                  # vide = $DISPLAY, ou 127.0.0.1:0.0 sous Windows

[[tunnels]]
id = "01JA..."                    # ULID
name = "Adonis"
host = "nova"                     # alias de ~/.ssh/config
kind = "local"                    # local | remote | dynamic
listen_port = 3333
# listen_address = "0.0.0.0"      # surcharge du défaut
# target_host = "127.0.0.1"
# target_port = 3333              # défaut = listen_port
tags = ["dev", "adonis"]

[hosts.nova]                      # surcharges propres à l'app, par alias
x11_display = "198.51.100.135:0.0"

[[x11_launchers]]
host = "nova"
name = "VS Code"
command = "code --wait"
```

L'état courant est stocké à part dans `state.toml` (dossier state/data) pour ne pas polluer le fichier édité à la main : `running` (tunnels démarrés par l'utilisateur, pas vidé par « Tout arrêter et quitter », relancés si `restore_on_launch`) et `keyring_entries` (passphrases déposées dans le trousseau, pour pouvoir les oublier).

---

## 5. X11

Objectif : remplacer le combo VcXsrv + `ssh -X` qui casse sous Windows faute de `xauth`.

- **Ancre X11 par host** : un canal session avec `request_x11` (cookie `MIT-MAGIC-COOKIE-1` factice aléatoire) qui exécute `echo $DISPLAY; exec sleep <long>` côté serveur. L'UI affiche le `DISPLAY` distant (ex. `localhost:10.0`) avec un bouton copier `export DISPLAY=...`. Toutes les sessions distantes de l'utilisateur peuvent l'utiliser : sshd inscrit le cookie dans le `~/.Xauthority` distant.
- **Lanceurs X11** : commandes enregistrées par host (`[[x11_launchers]]`) exécutées dans une session avec X11.
- **Canaux entrants** (`server_channel_open_x11`) : connexion au display local (`hosts.<alias>.x11_display` > `settings.x11_display` > `$DISPLAY`), en TCP `6000+n` ou socket Unix `/tmp/.X11-unix/Xn`.
- **Cookies** : vérification du cookie factice dans le paquet de setup X11, remplacement par le vrai cookie lu dans le `Xauthority` local (parser binaire maison) ; sans cookie local (VcXsrv « Disable access control »), auth vidée. Plus besoin du binaire `xauth`.
- Mode *trusted* uniquement. **Avertissement dans l'UI** : le X11 forwarding donne au serveur un accès complet au display local (capture clavier/écran possible). À n'activer que vers des hôtes de confiance.

---

## 6. Protocole IPC

JSON lines. Handshake `hello { protocol_version, client_kind: ui | cli }`.

- Requêtes : `{ "id": n, "method": "...", "params": {...} }` → `{ "id": n, "result": ... }` | `{ "id": n, "error": { "code", "message" } }`
- Événements : `{ "event": "...", "data": {...} }` (abonnement implicite pour les clients `ui`)

| Méthodes | |
|---|---|
| `state.snapshot` | tunnels + états + santé + métriques + settings + statut du tray |
| `tunnel.create` / `update` / `delete` / `start` / `stop` / `restart` | |
| `tag.start` / `tag.stop` | |
| `external.adopt` / `kill` / `ignore` | `import` passe par le formulaire de création prérempli (`tunnel.create`) |
| `hosts.list` / `hosts.resolve` | alias de `ssh_config` + config résolue (équivalent `ssh -G`) |
| `settings.update` | |
| `credential.respond` / `hostkey.respond` | réponses aux prompts (`remember` pour une passphrase) |
| `secrets.forget` | oublie les passphrases mémorisées (mémoire + trousseau) |
| `x11.start` / `x11.stop` / `x11.launch` | |
| `import.ssh_config` / `import.script` | |
| `daemon.quit { force }` | refusé sans `force` si des tunnels tournent → l'UI demande confirmation |
| `ui.show` | utilisé par la 2ᵉ instance et le tray : `show.requested` aux UI connectées, sinon lancement de `tunneler-ui` |

Événements : `externals.changed`, `tunnel.changed`, `tunnel.metrics` (throttlé), `tunnel.log`, `credential.request`, `hostkey.request`, `prompt.resolved`, `config.reloaded`, `config.invalid`, `quit.requested`, `show.requested`, `tray.changed`.

---

## 7. UI / UX

- **Style** : sombre/clair selon le système (`useThemeStore` / `ThemeToggle` de la lib), dense et calme (esprit Linear/Raycast), composants `@minimalstuff/ui` + utilitaires UnoCSS, police système + mono pour les ports.
- **Composants absents de la lib, construits dans l'app** : drawer (panneau latéral), toasts + annonceur `aria-live`, palette de commandes, liste de tunnels navigable (roving tabindex), badge d'état icône + texte. Ceux qui sont génériques (drawer, toasts, palette) sont candidats à remonter dans `@minimalstuff/ui` une fois stabilisés.
- **Layout** : sidebar (tags avec toggle d'ensemble + compteur actifs/total, hosts, « Externes »), liste principale groupée par host, une ligne par tunnel : switch, nom, `3333 → nova:3333`, badge d'état, pastille de santé, mini-métriques.
- **Panneau détail** (drawer « Détails », séparé du drawer d'édition) : état, logs live, métriques, actions (restart, modifier, ouvrir dans le navigateur, copier la commande `ssh` équivalente).
- **Création rapide** : formulaire minimal (host, port) ; le port cible suit le port d'écoute tant qu'il n'est pas modifié ; bind pré-rempli par le défaut, surchargeable.
- **Palette de commandes** `Ctrl+K` (motif combobox + listbox, raccourcis via `useGlobalHotkeysStore`) : démarrer/arrêter tunnel ou tag, ouvrir, créer.
- **Prompts modaux** : passphrase/mot de passe, clé d'hôte inconnue, confirmation de sortie.
- **Tray** : icône selon l'état global (gris = rien, vert = actifs OK, orange = reconnexion/cible down, rouge = erreur) — chaque état a aussi une **forme/pictogramme distinct** et un tooltip textuel (« 3 tunnels actifs, 1 en reconnexion »). Menu : tags (toggle), *Ouvrir Tunneler*, *Quitter*.
- **Ouvrir dans le navigateur** : `http://localhost:<port>` pour un tunnel local actif, depuis le menu de ligne, le panneau détail ou la palette (pas de clic sur le port : la liste garde un arrêt de tabulation par contrôle). L'URL est construite côté natif, limitée aux adresses locales/IP.

---

## 8. Accessibilité

Cible : **WCAG 2.2 niveau AA** pour l'UI ; CLI conforme aux conventions `NO_COLOR` / non-TTY. Exigence transverse : chaque phase qui touche l'UI ou la CLI inclut ses critères d'accessibilité dans sa définition de « fait ».

### UI
- **Clavier complet** : toute action atteignable sans souris, ordre de tabulation logique, focus toujours visible (anneau ≥ 3:1), navigation fléchée dans les listes (roving tabindex), raccourcis documentés dans une aide (`?`) et jamais seule voie d'accès.
- **Sémantique** : HTML natif d'abord (`button`, `form`, `label`, `table`/`list`), landmarks (`nav`, `main`, `aside`), titres hiérarchisés. Composants `@minimalstuff/ui` pour les motifs ARIA complexes (modal, menu, combobox, switch, tabs, tooltip) ; tout écart d'accessibilité constaté dans la lib est corrigé dans la lib, pas contourné dans l'app.
- **Noms accessibles explicites** : un switch annonce « Tunnel Adonis, 3333 vers nova:3333, actif », pas juste « on ». Boutons icône toujours labellisés.
- **Jamais la couleur seule** : chaque état/santé = icône + texte (badge « Reconnexion 2/∞ », « Cible injoignable »). Contraste texte ≥ 4.5:1, composants ≥ 3:1, vérifiés dans les deux thèmes.
- **Annonces dynamiques** : région `aria-live="polite"` pour les changements d'état (regroupés/throttlés pour éviter le spam), `assertive` réservé aux erreurs bloquantes. Les prompts (passphrase, clé d'hôte) prennent le focus.
- **Modales** : focus piégé, `Échap` ferme (sauf alerte clé changée qui exige un choix explicite), retour du focus à l'élément déclencheur.
- **Formulaires** : labels visibles, erreurs liées (`aria-describedby`, `aria-invalid`), message d'erreur textuel précis, pas de validation uniquement au survol.
- **Préférences système** : `prefers-reduced-motion` (aucune animation essentielle), `prefers-color-scheme`, `forced-colors` (mode contraste élevé Windows), zoom 200 % / grand texte sans perte de contenu ni scroll horizontal.
- **Cibles** : zones cliquables ≥ 24×24 px (WCAG 2.5.8).
- **Lecteurs d'écran testés** : NVDA (Windows/WebView2), Orca (Linux/WebKitGTK).

### Tray
- Libellés de menu textuels incluant l'état (« dev — 3/5 actifs »), éléments cochables pour les tags, tooltip résumant l'état global.
- Le tray n'est jamais la seule voie : tout est faisable depuis l'UI et la CLI.

### CLI
- **Pas d'information portée par la couleur seule** : symboles + mots (`✔ actif`, `✖ échec`), avec repli ASCII si le terminal n'est pas UTF-8.
- Respect de `NO_COLOR`, `TERM=dumb`, option `--color auto|always|never` ; couleurs, spinners et réécritures de ligne désactivés automatiquement hors TTY.
- Pas de spinner comme seul signal de progression : messages textuels d'étape.
- Sorties : format par défaut lisible ligne à ligne (pas de tableaux en caractères de dessin qui se lisent mal au lecteur d'écran), `--json` stable pour les scripts.
- Erreurs : message clair sur stderr avec la cause et l'action suggérée, codes de sortie documentés.
- Confirmations interactives contournables (`--yes`), jamais de prompt bloquant hors TTY.
- `--help` complet avec exemples, aide par sous-commande.

---

## 9. Phases

Le détail coché est dans [`TASKS.md`](./TASKS.md). Les phases UI/CLI ne sont closes qu'avec leurs critères d'accessibilité (§8) validés.

| Phase | Contenu | Livrable vérifiable |
|---|---|---|
| 0 | Bootstrap workspace, outillage, CI, `CLAUDE.md` projet | `cargo build`, `clippy`, `rustfmt`, `oxlint`, `oxfmt --check` verts |
| 1 | Crate `ssh-config` complète | Tests différentiels contre `ssh -G` sur un corpus de configs |
| 2 | Crate `known-hosts` | Tests contre fichiers générés par `ssh-keygen -H` / `ssh-keyscan` |
| 3 | Engine : connexion + auth + host key | Connexion à un serveur russh de test in-process (clé, agent, password, ProxyJump) |
| 4 | Engine : forwards locaux, états, reconnexion, santé, métriques, store | Tests d'intégration : toggle d'un tunnel sans couper les autres, reconnexion après coupure |
| 5 | IPC + daemon + CLI + single instance | `tunneler up dev` / `status` / `down` contre le daemon |
| 6 | UI Tauri MVP | CRUD, toggle, tags, settings, prompts, confirmation de sortie |
| 7 | Tray + lancement de l'UI à la demande | Daemon seul en tray, UI ouverte/fermée sans impact sur les tunnels |
| 8 | Tunnels externes | Tunnel lancé par le script PS/`ssh` détecté, adopté, tué |
| 9 | Forwards remote + dynamic (SOCKS5) | `curl --socks5` à travers le tunnel |
| 10 | X11 | `xeyes` distant affiché sur VcXsrv sans `xauth` |
| 11 | Confort : imports, palette, logs, ouvrir navigateur | Import du script PS existant en un collage |
| 12 | Autostart, tags au lancement, restore, trousseau OS | Redémarrage de session → tunnels restaurés |
| 13 | Packaging, mesure RAM, release | Zip portable Windows, AppImage/tar.gz Linux, mesures RSS documentées |

---

## 10. Qualité

- Rust : tests unitaires co-localisés (`#[cfg(test)]`), tests d'intégration dans `tests/` avec un serveur SSH russh in-process (pas de dépendance à un sshd système), `clippy -D warnings`, `rustfmt`.
- `ssh-config` : tests différentiels contre `ssh -G` (activés si `ssh` présent).
- Frontend : Vitest + Testing Library (requêtes par rôle/nom accessible en priorité), oxlint (preset `@minimalstuff/tooling` `react: true`, type-aware, + plugin `jsx-a11y`), oxfmt, `tsc --noEmit`.
- Accessibilité automatisée : `axe-core` (via `vitest-axe`) sur chaque composant et écran ; échec CI sur violation. Tests CLI de sortie avec `NO_COLOR=1` et hors TTY.
- Accessibilité manuelle : checklist par phase UI (clavier seul, NVDA, Orca, zoom 200 %, contraste élevé Windows) — l'automatique ne couvre qu'une partie des critères.
- CI GitHub Actions : matrice Linux / Windows / macOS.

## 11. Risques

| Risque | Mitigation |
|---|---|
| GNOME sans extension AppIndicator → pas de tray | Absence de propriétaire de `org.kde.StatusNotifierWatcher` détectée et suivie, bandeau dans l'UI, l'UI reste accessible via `tunneler` |
| RAM WebKitGTK élevée | UI à la demande (D2) ; mesurer en phase 13 |
| PID introuvable pour sockets d'autres utilisateurs (Linux) | État « process inconnu », actions limitées |
| Parité `ssh_config` avec OpenSSH (`Match exec`, `canonical`, `final`) | Tests différentiels ; directives non supportées listées et signalées dans l'UI |
| Sécurité IPC | Socket `0600` / named pipe ACL utilisateur, vérification `PeerCreds` |
| Sécurité X11 | Opt-in par host, avertissement explicite |
| Lacunes d'accessibilité dans `@minimalstuff/ui` | `axe` + passe manuelle sur chaque composant utilisé dès la phase 6 ; correctifs remontés dans la lib |
| Support lecteur d'écran inégal selon le webview (WebKitGTK + Orca moins mûr que WebView2 + NVDA) | Tester tôt (phase 6) sur les deux, privilégier HTML natif, CLI comme alternative complète |
