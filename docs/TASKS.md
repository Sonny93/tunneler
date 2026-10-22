# Tunneler — Tâches

Ce qui reste à faire, à valider ou à connaître du [plan](./PLAN.md). Une phase livrée sort de ce fichier : ce qui a changé une décision va dans `PLAN.md`, ce qui reste à valider ou manque encore va dans les sections ci-dessous, le reste vit dans l'historique git. Notes d'implémentation détaillées des phases 0 à 17 : `git show 88f2040:docs/TASKS.md`.

Légende : `[ ]` à faire · `[x]` fait · `[~]` en cours · `[-]` abandonné (avec raison)

**Définition de « fait » pour toute tâche UI/CLI** : utilisable au clavier seul, noms accessibles corrects, aucune info portée par la couleur seule, `axe` sans violation (UI) / sortie correcte avec `NO_COLOR=1` et hors TTY (CLI). Voir [PLAN §8](./PLAN.md#8-accessibilité).

---

## Phase 18 — Distribution (D16, PLAN §9)
- [x] 18.1 Mode portable : marqueur `portable` à côté de l'exécutable → `tunnels.toml` à côté de l'exe, `state.toml` + journaux + cache du webview dans `data/` ; même détection dans le daemon et l'UI (`ipc` ne dépend toujours pas d'`engine`) ; `--config` / `--state` prioritaires ; `daemon.paths` à jour ; marqueur livré dans le zip et le tar.gz
  - Note : détection dans `ipc` (`portable.rs`, voisin de `locate_companion_binary`), partagée par le daemon (`crates/tunneler/src/layout.rs`, aussi utilisé par les journaux et `purge`) et l'UI. Le cache du webview par défaut est `<dossier de données locales>/com.minimalstuff.tunneler` sous Windows (WebView2) et Linux (WebKitGTK), d'après `app_local_data_dir` de Tauri : la fenêtre `main` est déclarée avec `"create": false` et construite dans `setup` (`main_window.rs`, mêmes options) pour y passer `data_directory` (`data/webview`) en mode portable.
- [x] 18.2 `tunneler purge [--keep-config] [--dry-run] [--yes]` : arrêt du daemon, lancement au login, AUMID, journaux, cache du webview, puis config + état + passphrases du trousseau sauf `--keep-config` ; jamais les exécutables ni `~/.ssh` ; liste et confirmation sur TTY, code 5 hors TTY sans `--yes`, `--help` avec exemples, code de sortie documenté
  - Note : chaque trace est une variante de `PurgeTarget` (`cli/commands/purge/plan.rs`), l'AUMID de 18.4 compris. Un fichier donné par `--config` / `--state` (ou leurs variables) n'est jamais supprimé et la commande le dit ; les passphrases retirées sont celles du `state.toml` par défaut. Les éléments absents sont ignorés sans bruit (le lancement au login et les passphrases du trousseau, qu'on ne sonde pas, sont toujours listés).
- [x] 18.3 Installeur NSIS : `tauri.windows.conf.json` (cible `nsis`, `tunneler.exe` en `externalBin`, `installMode: "both"`, `startMenuFolder`, langue française, fichier de langue personnalisé pour la case de suppression), hooks (`quit --yes` avant installation et désinstallation, `purge --yes [--keep-config]` hors `$UpdateMode`)
  - Note : vérifié sur les sources de tauri-bundler 2.10.3 (celui du CLI 2.12.3), mais l'installeur n'a pas pu être construit hors Windows : à valider en CI et à la main (18.8). Les hooks tournent avant `CheckIfAppIsRunning`, `$UpdateMode` et `$DeleteAppDataCheckboxState` sont des variables globales visibles dans les deux sections. Le fichier de langue personnalisé (`app/src-tauri/windows/French.nsh`) est la copie de celui de Tauri avec le seul libellé `deleteAppData` changé, plus `{{product_name}}` remplacé par `${PRODUCTNAME}` (les fichiers de langue ne passent pas par Handlebars : le littéral s'afficherait). `tauri-build` copie le sidecar dans `target/<profil>/tunneler.exe` ; il faut donc l'avoir préparé (`binaries/tunneler-x86_64-pc-windows-msvc.exe`, ignoré par git) avant tout build du crate UI sous Windows : CI et workflow de release s'en chargent.
- [x] 18.4 Notifications Windows au nom de Tunneler : le daemon enregistre l'AUMID `com.minimalstuff.tunneler` (`HKCU\Software\Classes\AppUserModelId`, nom affiché + icône) et l'utilise pour ses toasts à la place de l'AUMID de PowerShell ; repli sur PowerShell si l'enregistrement échoue. À vérifier sous Windows qu'un toast s'affiche sans raccourci du menu Démarrer
  - Note : nom affiché seulement, pas d'`IconUri` (il exige une image sur disque, une trace de plus) : les toasts portent le nom « Tunneler » sans icône propre. Enregistré au démarrage des signaux de veille (`daemon/notification_identity/`), retiré par `purge` même avec `--keep-config`. Jamais exécuté sous Windows : affichage d'un toast sans raccourci du menu Démarrer à vérifier en 18.8.
- [x] 18.5 Paquet `.deb` : `bundle.linux.deb` (daemon par `files` → `/usr/bin/tunneler`, pas d'`externalBin`), entrée de menu, construit sur ubuntu-22.04
  - Note : cibles et `deb.files` dans `tauri.linux.conf.json` (fusionné par plateforme, comme le fichier Windows ; la cible `appimage` n'est plus dans `tauri.conf.json`). Le bundler produit `Tunneler_<v>_amd64.deb` (paquet `tunneler`, `Depends: libwebkit2gtk-4.1-0, libgtk-3-0`, entrée `Tunneler.desktop` qui lance `tunneler-ui`) : le workflow le renomme en `tunneler_<v>_amd64.deb`. Contenu vérifié avec `dpkg-deb -I/-c` (`/usr/bin/tunneler` et `/usr/bin/tunneler-ui`, icônes hicolor).
- [x] 18.6 Mises à jour : `tauri-plugin-updater`, clé de signature (secrets `TAURI_SIGNING_PRIVATE_KEY` + mot de passe, copie hors GitHub), `createUpdaterArtifacts`, `latest.json` produit par le job `release` ; réglage `check_for_updates` (défaut `true`) ; bandeau à l'ouverture de l'UI ; Windows installé : *Installer et redémarrer* avec confirmation si des tunnels tournent, progression textuelle, mode `passive` ; autres formes : lien vers la release. Vérification en échec (dépôt privé) journalisée sans message
  - Note : l'updater est piloté côté Rust (`app/src-tauri/src/update_commands.rs` : `check_for_update`, `install_update` avec un `Channel` de progression, `open_release_page`), la webview n'a aucune permission updater ni opener. L'action est décidée par `decide_update_action` (Windows installé seulement). L'installation télécharge et vérifie (`Update::download`), puis quitte le daemon (`quit { force }`), puis `Update::install` : NSIS en `/P /UPDATE /R` relance l'UI. Le manifeste ne porte que `windows-x86_64` et `linux-x86_64` (l'AppImage) : la vérification trouve donc une version sur toutes les formes Linux, qui n'offrent que le lien. Le canal de log de l'UI n'a pas d'abonné `tracing` : l'échec de vérification est émis (`warn`) mais n'écrit nulle part pour l'instant. Pas de `tauri-plugin-process` : l'installeur relance l'application. Jamais exécuté sous Windows ni contre une release publique (18.8).
- [x] 18.7 Workflow de release (installeur, `.deb`, signatures, `latest.json`, `SHA256SUMS`, noms de fichiers du PLAN §9) + README (trois formes, mode portable, `tunneler purge`, désinstallation Linux)
  - Fait : jobs Windows (installeur NSIS + zip) et Linux (`.deb` + tar.gz + AppImage) du workflow de release, `SHA256SUMS` inchangé, README (installation, désinstallation, mode portable). Signatures (`.sig` renommés comme leurs actifs) et `latest.json` (notes = corps de la release s'il existe) produits par le workflow, `SHA256SUMS` calculé après eux. Validé avec `actionlint` ; jamais exécuté en CI.
- [ ] 18.8 Validation à la main sous Windows : installation « pour moi » et « pour tous », raccourcis cochés ou non, mise à jour depuis la version précédente (tunnels restaurés), désinstallation avec et sans la case (aucune trace du tableau PLAN §9), installeur au clavier et avec NVDA ; sous Debian : `.deb` installé, lancé, mis à jour, retiré

## Phase 19 — Sons de veille synthétisés (D14, PLAN §« Veille »)
- [x] 19.1 Réglage `settings.standby_sound` (`system` \| `chime` \| `marimba` \| `pluck`, défaut `pluck`) : moteur (modèle, lecture/écriture du TOML, exemple du plan), DTO IPC (`standbySound`, défaut `pluck`), mapping du daemon
- [x] 19.2 Synthèse en Rust, sans dépendance (`standby_signal/synthesis.rs`) : carillon, marimba, corde pincée (Karplus-Strong, bruit xorshift à graines fixes), deux notes montantes au réveil et descendantes à l'échec, WAV 44,1 kHz mono 16 bits normalisé à 0,5, mis en cache par son et événement
  - Note : le son est celui des prototypes (mêmes notes, durées, écarts, fondu final de 20 ms) mais pas échantillon pour échantillon : le bruit vient d'un générateur maison, pas de Python.
- [x] 19.3 Lecture : `SoundPlayer::play(sound, event)`, lu à chaque événement ; Windows `PlaySoundW` en `SND_MEMORY` sur le tampon mis en cache ; Linux par l'entrée standard de `pw-play -` puis `paplay` (`launch::spawn_detached_with_input`, un thread nourrit et récolte le processus), repli sur les candidats `system`
- [x] 19.4 `settings.preview_sound { sound, event }` : requête IPC (contrat JSON figé, `METHOD_NAMES`), client `preview_sound`, handler du daemon, commande Tauri `preview_sound`
- [x] 19.5 Réglages de l'interface (Veille) : choix « Son » (corde pincée, carillon, marimba, son du système), toujours actif, avec l'indication « Choisissez « Son » ou « Les deux »… » quand le signal n'en joue pas ; boutons « Écouter le réveil » / « Écouter l'échec » sur la valeur du formulaire, même non enregistrée, erreur en toast via `useDaemonAction`

## Phase 20 — Retours d'interface et veille forcée (PLAN §« Veille », §7)
- [x] 20.1 Moteur : renvoyer en veille un tunnel réveillé (un, un tag, tous), connexions fermées, `is_awake` dans l'instantané ; IPC `tunnel.standby`, `tag.standby`, `tunnels.standby`, `isAwake`, événement `standby.requested` ; daemon (handler, `UiCoordinator`) ; commandes Tauri et passerelle
- [x] 20.2 Tray : *Mettre en veille les tunnels réveillés*, confirmation déléguée à l'UI si des connexions sont ouvertes
- [x] 20.3 UI : « Mettre en veille » sur un tunnel (menu, détail, palette), la sélection, un tag et « Tous les tunnels » (clic droit, palette), confirmation si des connexions sont ouvertes, écoute de `standby.requested`
  - Note : logique partagée dans `app/src/standby/` (`useSendToStandby`, `confirmStandby`) ; le tray n'envoie lui-même en veille que sans connexion ouverte, sinon il délègue la confirmation à l'UI. Barre de sélection avec le bouton en plus jamais vue à l'écran.
- [x] 20.4 Interrupteur de tag (sidebar, palette) : un tunnel en veille compte comme démarré
- [x] 20.5 Choix radio de plus de deux options toujours à la verticale ; groupe « Contrôle » dans le menu d'un tunnel ; icône de chaque vue du panneau latéral quand elle n'est pas sélectionnée

## Reste à valider à la main
- [~] 6.13 Zoom 200 % sans scroll horizontal dans une vraie webview (`prefers-reduced-motion` et mise en page empilée sous `md` faits)
- [~] 6.14 NVDA (Windows/WebView2), contraste élevé Windows, écoute Orca par un humain (passe clavier + arbre AT-SPI faite sous Linux le 2026-10-16)
- [~] 13.5 Audit WCAG 2.2 AA final sur les binaires de release, grille dans [`AUDIT_A11Y.md`](./AUDIT_A11Y.md)
- [ ] 10.6 X11 : VcXsrv (Windows) + Xorg/Xwayland (Linux) contre un vrai sshd (`xeyes` distant affiché sans `xauth`)
- [ ] `curl --socks5` à travers un tunnel dynamique vers un vrai sshd
- [ ] Rendu réel jamais vu par un humain : veille forcée (menu, barre de sélection, clic droit de « Tous les tunnels », entrée du tray), tiroirs (détails, import, X11, config d'hôte), palette, panneau replié, renommage/suppression de tag, formulaire « Ouverture », section « Applications », badges de veille, infobulles, clic droit, sélection multiple
- [ ] Entrée pour valider une invite de passphrase (jsdom ne soumet pas un formulaire dont le bouton est hors du `<form>`)
- [ ] Windows, code compilé en cross-clippy mais pas explicitement vérifié : named pipe et son ACL, tray (`winit`), `FreeConsole`, ressources MSVC, `Run` (`winreg`), Credential Manager, `TerminateProcess`, agent par named pipe + Pageant, `ProxyCommand`/`Match exec` via `cmd /C`, détection des applications, lancement détaché, toast, `PlaySoundW`
- [ ] 19.6 Sons de veille : les écouter sur un vrai Windows (dont avec « Notification » réglé sur « (Aucun) » dans le jeu de sons, et en `PlaySoundW` `SND_MEMORY`) et sur un vrai bureau Linux (PipeWire : `pw-play -`) ; `paplay` par l'entrée standard (non installé sur la machine de dev, jamais essayé)
- [ ] Linux : tray sous KDE ; notification D-Bus et sons de veille sur un vrai bureau ; mesure RAM de l'UI sur un vrai bureau (410 Mio PSS mesurés sous Xvfb en rendu logiciel)

## À corriger ailleurs
- [~] 6.16 `@minimalstuff/ui` : portail du `Menu` et du menu contextuel hors landmark (règle axe `region` désactivée dans `tunnel_list_selection.test.tsx`)

## Lacunes connues
Limites assumées à ce jour, candidates à une tâche si le besoin apparaît.

- **`ssh-config`** : ports nommés (`getservbyname`), `~user/`, contrôles de permissions des fichiers, `ForwardAgent no` suivi d'un chemin ; `~` développé avec `$HOME` (OpenSSH : home de passwd, différent en conteneur) ; `localnetwork`, `version`, `sessiontype`, `command` dans `Match` ne correspondent jamais ; `CanonicalizeHostname` traité comme `no`. Comportements Windows (`%PROGRAMDATA%`, `cmd /C`, globs) non vérifiés.
- **`known-hosts` / clés d'hôte** : certificats d'hôte refusés (`@cert-authority` inutilisable), révocation de la clé d'un certificat non vérifiée (seule la CA l'est).
- **Auth** : `AddKeysToAgent` non implémenté ; certificats utilisateur câblés sans test d'intégration ; seul le format de clé OpenSSH testé de bout en bout.
- **Sessions** : un bastion n'est pas partagé entre deux cibles (une connexion au saut par cible). Au rechargement, passer d'une valeur absente à la valeur par défaut explicite redémarre le tunnel.
- **Externes** : sans UI connectée, aucun scan périodique (`tunneler status` montre le dernier scan) ; la destination ssh n'est pas comparée ; pas de `adopt`/`kill`/`ignore` en CLI ; sockets d'un autre utilisateur sans pid sous Linux.
- **X11** : un hôte sans tunnel n'a pas de groupe, donc pas d'accès X11 dans l'UI ; display launchd de macOS non supporté ; ancres X11 non restaurées au lancement.
- **Import** : `$env:DISPLAY` d'un script n'est pas repris en `x11_display` ; pas d'import en CLI.
- **CLI** : heures de santé en UTC.
- **Store** : un `tunnels.toml` en CRLF réécrit n'est pas garanti identique hors des lignes touchées.
- **Accessibilité** : bordures des contrôles de la lib sous 3:1 (WCAG 1.4.11, écart accepté le 2026-10-19) ; le bouton du menu d'actions est exposé « combo box » par WebKitGTK (`aria-haspopup`).
- **macOS** (non maintenu) : pas de tray, pas de pid du pair IPC.
- **Tests** : un échec isolé non identifié de la suite Rust en phase 15 (non reproduit).
