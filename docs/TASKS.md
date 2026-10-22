# Tunneler — Tâches

Ce qui reste à faire, à valider ou à connaître du [plan](./PLAN.md). Une phase livrée sort de ce fichier : ce qui a changé une décision va dans `PLAN.md`, ce qui reste à valider ou manque encore va dans les sections ci-dessous, le reste vit dans l'historique git. Notes d'implémentation détaillées des phases 0 à 17 : `git show 6f613f3:docs/TASKS.md`.

Légende : `[ ]` à faire · `[x]` fait · `[~]` en cours · `[-]` abandonné (avec raison)

**Définition de « fait » pour toute tâche UI/CLI** : utilisable au clavier seul, noms accessibles corrects, aucune info portée par la couleur seule, `axe` sans violation (UI) / sortie correcte avec `NO_COLOR=1` et hors TTY (CLI). Voir [PLAN §8](./PLAN.md#8-accessibilité).

---

## Phase 18 — Distribution (D16, PLAN §9)
- [x] 18.1 Mode portable : marqueur `portable` à côté de l'exécutable → `tunnels.toml` à côté de l'exe, `state.toml` + journaux + cache du webview dans `data/` ; même détection dans le daemon et l'UI (`ipc` ne dépend toujours pas d'`engine`) ; `--config` / `--state` prioritaires ; `daemon.paths` à jour ; marqueur livré dans le zip et le tar.gz
  - Note : détection dans `ipc` (`portable.rs`, voisin de `locate_companion_binary`), partagée par le daemon (`crates/tunneler/src/layout.rs`, aussi utilisé par les journaux et `purge`) et l'UI. Le cache du webview par défaut est `<dossier de données locales>/com.minimalstuff.tunneler` sous Windows (WebView2) et Linux (WebKitGTK), d'après `app_local_data_dir` de Tauri : la fenêtre `main` est déclarée avec `"create": false` et construite dans `setup` (`main_window.rs`, mêmes options) pour y passer `data_directory` (`data/webview`) en mode portable.
- [x] 18.2 `tunneler purge [--keep-config] [--dry-run] [--yes]` : arrêt du daemon, lancement au login, AUMID, journaux, cache du webview, puis config + état + passphrases du trousseau sauf `--keep-config` ; jamais les exécutables ni `~/.ssh` ; liste et confirmation sur TTY, code 5 hors TTY sans `--yes`, `--help` avec exemples, code de sortie documenté
  - Note : l'AUMID n'existe pas encore (18.4) : `PurgeTarget` (`cli/commands/purge/plan.rs`) est le point d'extension. Un fichier donné par `--config` / `--state` (ou leurs variables) n'est jamais supprimé et la commande le dit ; les passphrases retirées sont celles du `state.toml` par défaut. Les éléments absents sont ignorés sans bruit (le lancement au login et les passphrases du trousseau, qu'on ne sonde pas, sont toujours listés).
- [ ] 18.3 Installeur NSIS : `tauri.windows.conf.json` (cible `nsis`, `tunneler.exe` en `externalBin`, `installMode: "both"`, `startMenuFolder`, langue française, fichier de langue personnalisé pour la case de suppression), hooks (`quit --yes` avant installation et désinstallation, `purge --yes [--keep-config]` hors `$UpdateMode`)
- [ ] 18.4 Notifications Windows au nom de Tunneler : le daemon enregistre l'AUMID `com.minimalstuff.tunneler` (`HKCU\Software\Classes\AppUserModelId`, nom affiché + icône) et l'utilise pour ses toasts à la place de l'AUMID de PowerShell ; repli sur PowerShell si l'enregistrement échoue. À vérifier sous Windows qu'un toast s'affiche sans raccourci du menu Démarrer
- [ ] 18.5 Paquet `.deb` : `bundle.linux.deb` (daemon par `files` → `/usr/bin/tunneler`, pas d'`externalBin`), entrée de menu, construit sur ubuntu-22.04
- [ ] 18.6 Mises à jour : `tauri-plugin-updater`, clé de signature (secrets `TAURI_SIGNING_PRIVATE_KEY` + mot de passe, copie hors GitHub), `createUpdaterArtifacts`, `latest.json` produit par le job `release` ; réglage `check_for_updates` (défaut `true`) ; bandeau à l'ouverture de l'UI ; Windows installé : *Installer et redémarrer* avec confirmation si des tunnels tournent, progression textuelle, mode `passive` ; autres formes : lien vers la release. Vérification en échec (dépôt privé) journalisée sans message
- [ ] 18.7 Workflow de release (installeur, `.deb`, signatures, `latest.json`, `SHA256SUMS`, noms de fichiers du PLAN §9) + README (trois formes, mode portable, `tunneler purge`, désinstallation Linux)
- [ ] 18.8 Validation à la main sous Windows : installation « pour moi » et « pour tous », raccourcis cochés ou non, mise à jour depuis la version précédente (tunnels restaurés), désinstallation avec et sans la case (aucune trace du tableau PLAN §9), installeur au clavier et avec NVDA ; sous Debian : `.deb` installé, lancé, mis à jour, retiré

## Reste à valider à la main
- [~] 6.13 Zoom 200 % sans scroll horizontal dans une vraie webview (`prefers-reduced-motion` et mise en page empilée sous `md` faits)
- [~] 6.14 NVDA (Windows/WebView2), contraste élevé Windows, écoute Orca par un humain (passe clavier + arbre AT-SPI faite sous Linux le 2026-10-16)
- [~] 13.5 Audit WCAG 2.2 AA final sur les binaires de release, grille dans [`AUDIT_A11Y.md`](./AUDIT_A11Y.md)
- [ ] 10.6 X11 : VcXsrv (Windows) + Xorg/Xwayland (Linux) contre un vrai sshd (`xeyes` distant affiché sans `xauth`)
- [ ] `curl --socks5` à travers un tunnel dynamique vers un vrai sshd
- [ ] Rendu réel jamais vu par un humain : tiroirs (détails, import, X11, config d'hôte), palette, panneau replié, renommage/suppression de tag, formulaire « Ouverture », section « Applications », badges de veille, infobulles, clic droit, sélection multiple
- [ ] Entrée pour valider une invite de passphrase (jsdom ne soumet pas un formulaire dont le bouton est hors du `<form>`)
- [ ] Windows, code compilé en cross-clippy mais pas explicitement vérifié : named pipe et son ACL, tray (`winit`), `FreeConsole`, ressources MSVC, `Run` (`winreg`), Credential Manager, `TerminateProcess`, agent par named pipe + Pageant, `ProxyCommand`/`Match exec` via `cmd /C`, détection des applications, lancement détaché, toast, `PlaySoundW`
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
