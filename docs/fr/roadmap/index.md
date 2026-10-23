# Feuille de route

Ce que fait Tunneler aujourd'hui, ce qui vient ensuite, ce qu'il faut tester sur de vraies machines et ce qui manque encore. Les raisons des choix principaux sont sur la page [Décisions de conception](./decisions).

## Livré

### Tunnels
- Redirections locales (`-L`), distantes (`-R`) et dynamiques SOCKS5 (`-D`).
- Une session SSH par hôte, partagée par tous ses tunnels et fermée peu après l'arrêt du dernier.
- Reconnexion automatique avec délai exponentiel et gigue. Une coupure met les tunnels en reconnexion au lieu de les faire échouer.
- État de chaque tunnel, octets échangés, connexions actives et journal en direct.
- Une santé passive : une connexion refusée vers la cible la marque injoignable. Une vérification ponctuelle à la demande (`tunneler check`, ou « Tester la cible » dans l'interface).
- Tunnels temporaires, lancés depuis la CLI avec la syntaxe d'OpenSSH (`tunneler up <hôte> -L 3333`), conservés tant que le daemon tourne et enregistrables dans la config.
- Veille : un tunnel garde son port local et n'ouvre la session SSH qu'à la première connexion, puis retourne en veille après un temps d'inactivité. Notification ou son facultatifs au réveil ou quand le réveil échoue. Les tunnels réveillés peuvent être renvoyés en veille à la main, un par un, par tag ou tous ensemble.
- Redirection X11 sans le binaire `xauth`, avec une ancre par hôte et des lanceurs enregistrés.

### SSH
- Client intégré : aucun `ssh` système requis.
- Un analyseur de `ssh_config` complet, comparé à `ssh -G` : `Host`, `Match`, `Include`, jetons, `ProxyJump`, `ProxyCommand`, options de keepalive.
- Authentification dans l'ordre d'OpenSSH : agent (socket Unix, named pipe OpenSSH ou Pageant), fichiers de clé (OpenSSH, PKCS#8, PuTTY), keyboard-interactive, mot de passe.
- Les passphrases de clé peuvent être retenues pour la session ou dans le trousseau du système. Les mots de passe ne sont jamais retenus.
- Un analyseur de `known_hosts` avec jokers, négation, entrées hachées, `@revoked` et `StrictHostKeyChecking`. Une clé d'hôte qui change est un échec définitif.

### Tunnels lancés hors de Tunneler
- Détection des processus `ssh` qui tiennent le port d'un tunnel configuré, et des tunnels `ssh` qui ne sont pas configurés du tout.
- Les adopter, les tuer, les ignorer ou les importer comme tunnel configuré.
- Les ports tenus par d'autres programmes apparaissent comme des conflits, avec le programme et son pid.

### Interface
- Icône dans la zone de notification dont la forme et l'infobulle disent l'état global, avec un menu pour activer les tags.
- Fenêtre à la demande : le processus s'arrête à la fermeture de la fenêtre, les tunnels continuent dans le daemon.
- Liste de tunnels groupée par hôte, avec tags, sélection multiple, menus contextuels et palette de commandes (Ctrl+K).
- Panneau de détails avec journaux et métriques en direct, import depuis `ssh_config` et depuis des scripts `ssh -L` collés. Les tiroirs glissent à l'ouverture et à la fermeture.
- Une fenêtre de réglages à onglets verticaux (Général, Connexion, Veille, Ouverture, Secrets), un seul « Enregistrer » pour tout, y compris vos applications, le nombre d'erreurs sur chaque onglet, une confirmation avant d'abandonner les changements quelle que soit la façon de fermer la fenêtre (Annuler, Échap, un clic à l'extérieur ou le bouton de fermeture), et une commande par onglet dans la palette.
- Ouverture d'un tunnel dans le navigateur ou dans une application choisie d'après le port cible (VNC, RDP, toute commande que vous ajoutez). Une application détectée peut être personnalisée, par exemple pour pointer vers une copie portable.
- Cette documentation, en anglais et en français, ouverte depuis l'application avec F1.
- Thèmes clair et sombre qui suivent le système.
- Anglais et français, pour la fenêtre, l'icône de la zone de notification et les notifications. La langue suit le système ou se choisit dans les réglages.

### Ligne de commande
- `tunneler up`, `down`, `status`, `list`, `open`, `check`, `quit` et `purge`, avec sortie `--json` et codes de sortie documentés.
- Les tunnels se désignent par port, par `hôte:port` ou par tag.

### Distribution
- Installeur Windows (par utilisateur ou pour la machine), paquet Debian, zip et tar.gz portables, et une AppImage de l'interface.
- Mode portable : tout reste à côté des exécutables.
- `tunneler purge` retire toutes les traces laissées sur la machine.
- Mises à jour signées depuis les versions GitHub, appliquées automatiquement avec l'installeur Windows et proposées par un lien pour les autres formes.
- Mémoire du daemon : environ 17 à 25 Mio. L'interface n'en consomme que fenêtre ouverte.

## Prochainement

- Ce que la section [À tester](#a-tester) fera ressortir.

## À tester

L'essentiel du code tourne dans des tests automatiques contre un serveur SSH en processus. Ce qui manque, ce sont de vraies machines. Si vous essayez l'un de ces points, un ticket avec le résultat, bon ou mauvais, aide.

### Windows
- **Installeur.** Installer pour l'utilisateur courant puis pour tous les utilisateurs, avec et sans raccourcis. Mettre à jour depuis la version précédente (les tunnels doivent revenir). Désinstaller avec et sans la case « Supprimer aussi mes tunnels, réglages et passphrases » : il ne doit rien rester.
- **Tout ce que fait le daemon sous Windows.** Il compile et passe le lint, mais tous les chemins n'ont pas été exécutés : le named pipe, la zone de notification, le lancement à l'ouverture de session, le gestionnaire d'identifiants, l'agent OpenSSH et Pageant, `ProxyCommand` et `Match exec`, la détection des clients VNC et RDP, le lancement détaché.
- **Notifications.** Un toast doit s'afficher au nom de Tunneler sans raccourci du menu Démarrer.
- **Sons de veille.** Tous les écouter, y compris avec le son « Notification » réglé sur « (Aucun) » dans le jeu de sons de Windows.
- **Lecteur d'écran et affichage.** NVDA avec l'interface, le mode contraste élevé de Windows, et un zoom à 200 % sans défilement horizontal.

### Linux
- **Paquet Debian.** Installer, lancer, mettre à jour et retirer le `.deb`.
- **Sons et notifications.** Les sons de veille sur un vrai bureau avec PipeWire (`pw-play -`), et avec PulseAudio seul (`paplay`, jamais essayé). Les notifications D-Bus.
- **Zone de notification sous KDE.**
- **Lecteur d'écran.** Orca, écouté par une personne. Le passage au clavier et l'arbre d'accessibilité ont été vérifiés.
- **Mémoire.** L'interface pèse environ 410 Mio PSS sous Xvfb en rendu logiciel. Un vrai bureau avec GPU devrait en prendre moins : une mesure le confirmerait.

### Les deux systèmes
- **X11.** VcXsrv sous Windows et Xorg ou Xwayland sous Linux, contre un vrai `sshd` : un `xeyes` distant doit s'afficher sans `xauth`.
- **SOCKS.** `curl --socks5` à travers un tunnel dynamique vers un vrai `sshd`.
- **L'interface telle qu'on la voit.** La veille forcée (menu, barre de sélection, clic droit sur « Tous les tunnels », entrée de la zone de notification), les tiroirs, la fenêtre de réglages et ses onglets, la palette de commandes, le panneau replié, le renommage et la suppression de tag, le formulaire « Ouverture » et la section « Applications », les badges de veille, les infobulles, le clic droit et la sélection multiple.
- **Fermer la fenêtre des réglages avec des changements non enregistrés.** Échap, un clic à l'extérieur et le bouton de fermeture doivent chacun demander « Abandonner vos modifications ? ». « Continuer la modification » doit remettre le focus sur le champ en cours d'édition.
- **Invite de passphrase.** Entrée doit la valider. L'environnement de test ne peut pas le vérifier.
- **Passe d'accessibilité sur les binaires de release.** WCAG 2.2 AA, sur les deux systèmes. En plus des points ci-dessus : visibilité de l'anneau de focus dans les deux thèmes, les flèches et Entrée dans la palette, l'invite de passphrase qui prend le focus, les erreurs de champ lues quand le champ reçoit le focus, les cases et avertissements de l'import, le thème clair ou sombre du système, la taille des cibles, et la CLI avec `NO_COLOR=1`, `TERM=dumb` et sans terminal. Les [règles d'accessibilité](/fr/contributing/accessibility) disent ce qui est attendu.

## Limites connues

### SSH
- `ssh_config` : les ports nommés, les chemins `~user/`, les contrôles de permissions de fichiers et `CanonicalizeHostname` ne sont pas gérés. `localnetwork`, `version`, `sessiontype` et `command` ne correspondent jamais dans une ligne `Match`. `~` se développe depuis `$HOME`, pas depuis l'entrée passwd. Les comportements propres à Windows (`%PROGRAMDATA%`, `cmd /C`, globs) ne sont pas vérifiés.
- Les certificats d'hôte sont refusés : les entrées `@cert-authority` sont inutilisables. La révocation de la clé contenue dans un certificat n'est pas vérifiée, seule celle de la CA l'est.
- `AddKeysToAgent` n'est pas implémenté. Les certificats utilisateur sont branchés sans test d'intégration. Seul le format de clé OpenSSH est testé de bout en bout.
- Un hôte de rebond n'est pas partagé entre deux cibles : chaque cible ouvre sa propre connexion vers lui.

### Tunnels
- La veille ne marche que pour les tunnels locaux et dynamiques. Un tunnel distant écoute côté serveur, il lui faut donc une session.
- Il n'y a pas de sonde de santé périodique : une cible en panne ne se remarque que quand une connexion échoue ou quand vous demandez une vérification.
- Recharger une config où un réglage passe d'absent à sa valeur par défaut écrite explicitement redémarre le tunnel.
- X11 : un hôte sans tunnel n'a pas de groupe, donc pas d'accès X11 dans l'interface. Le display de launchd sous macOS n'est pas géré, et les ancres X11 ne sont pas restaurées au lancement.

### Tunnels lancés ailleurs
- Sans interface ouverte, il n'y a pas de scan périodique : `tunneler status` montre le dernier.
- La destination de `ssh` n'est pas comparée pour associer un tunnel.
- Sous Linux, les sockets d'un autre utilisateur n'ont pas de pid sans root.
- Il n'y a pas de `adopt`, `kill` ni `ignore` en ligne de commande.

### Interface et ligne de commande
- L'import ne reprend pas le `$env:DISPLAY` d'un script dans `x11_display`, et il n'y a pas d'import en ligne de commande.
- Les heures de santé en ligne de commande sont en UTC.
- Un `tunnels.toml` en fins de ligne CRLF n'est pas garanti identique après réécriture en dehors des lignes que Tunneler modifie.
- Les bordures de certains contrôles de la bibliothèque de composants sont sous 3:1 de contraste (WCAG 1.4.11). C'est accepté pour l'instant.
- WebKitGTK expose le bouton du menu d'actions comme une liste déroulante (« combo box »).
- Les choix « English » et « Français » du réglage de langue ne portent pas leur langue pour les lecteurs d'écran : la bibliothèque de composants ne permet pas encore de poser `lang` sur une option.
- Les toasts Windows portent le nom Tunneler mais pas d'icône.
- Les binaires et installeurs ne sont pas signés : Windows SmartScreen avertit au premier lancement.
- macOS n'est pas maintenu : ni paquet, ni CI, ni zone de notification.
