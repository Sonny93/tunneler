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
- Veille : un tunnel garde son port local et n'ouvre la session SSH qu'à la première connexion, puis retourne en veille après un temps d'inactivité. Notification ou son facultatifs au réveil ou quand le réveil échoue, avec un réglage de volume pour les sons synthétisés. Les tunnels réveillés peuvent être renvoyés en veille à la main, un par un, par tag ou tous ensemble.
- Vérification du serveur X : avant de démarrer X11 ou un lanceur, Tunneler cherche un serveur X sur le display local. S'il n'y en a pas, il peut démarrer VcXsrv ou Xming s'ils sont installés (demander, toujours ou jamais), ou avertit que les fenêtres n'apparaîtront pas.
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
- Icône dans la zone de notification : le logo de Tunneler, coloré selon l'état global, avec une infobulle qui le résume et un menu pour activer les tags.
- Fenêtre à la demande : le processus s'arrête à la fermeture de la fenêtre, les tunnels continuent dans le daemon.
- Liste de tunnels groupée par hôte, avec tags, sélection multiple, menus contextuels et palette de commandes (Ctrl+K).
- Panneau de détails avec journaux et métriques en direct, tiroir « Importer et exporter » (`ssh_config`, scripts `ssh -L` collés, devslot et fichiers). Les tiroirs glissent à l'ouverture et à la fermeture.
- Une fenêtre de réglages à onglets verticaux (Général, Connexion, Veille, Ouverture, Secrets), un seul « Enregistrer » pour tout, y compris vos applications, le nombre d'erreurs sur chaque onglet, une confirmation avant d'abandonner les changements quelle que soit la façon de fermer la fenêtre (Annuler, Échap, un clic à l'extérieur ou le bouton de fermeture), et une commande par onglet dans la palette.
- Ouverture d'un tunnel dans le navigateur ou dans une application choisie d'après le port cible (VNC, RDP, toute commande que vous ajoutez). Une application détectée peut être personnalisée, par exemple pour pointer vers une copie portable. Un réglage fait ouvrir dans le navigateur les tunnels sans application enregistrée, et un double-clic sur une ligne ouvre le tunnel.
- Cette documentation, en anglais et en français, ouverte depuis l'application avec F1.
- Thèmes clair et sombre qui suivent le système.
- Anglais et français, pour la fenêtre, l'icône de la zone de notification et les notifications. La langue suit le système ou se choisit dans les réglages.

### Ligne de commande
- `tunneler up`, `down`, `status`, `list`, `open`, `check`, `export`, `import`, `devslot`, `quit` et `purge`, avec sortie `--json` et codes de sortie documentés.
- `tunneler desktop-entry install` donne à l'archive Linux une entrée de menu des applications et des icônes ; le daemon la garde pointée sur le `tunneler-ui` courant, et `tunneler purge` la supprime.
- Les tunnels se désignent par port, par `hôte:port` ou par tag.

### Export et import
- Déplacer ses tunnels vers un autre ordinateur : `tunneler export` et l'onglet « Fichier » du tiroir « Importer et exporter » écrivent les tunnels, avec leurs tags, leurs options d'ouverture et leur veille, dans un fichier, et `tunneler import` ou le même onglet le relit avec un aperçu. Voir [Déplacer ses tunnels vers un autre ordinateur](/fr/guide/export-import).
- L'import ne fait qu'ajouter : un tunnel que vous avez déjà n'est jamais modifié ni supprimé. Les tunnels identiques et les conflits de port sont ignorés et nommés, les hôtes peuvent être remplacés, une application absente ici est abandonnée, et les tunnels risqués démarrent décochés. Un `tunnels.toml` brut est accepté aussi.
- La détection des conflits (déjà configuré, port déjà utilisé, répété dans l'import) vit dans le daemon et sert toutes les sources d'import : le fichier, `ssh_config`, les scripts collés et devslot.
- Règle sur les noms d'hôte : un alias qui n'est pas fait de lettres, de chiffres et de `. _ - : @ [ ]`, ou qui commence par `-`, est refusé à la création ou à l'import d'un tunnel, et jamais résolu ni connecté (plus strict que la règle d'OpenSSH 9.6, CVE-2023-51385).

### devslot
- Découverte depuis le tiroir « Importer et exporter » : lister les instances d'un hôte avec `ds ls --json`, démarrer des tunnels vers leurs ports (temporaires ou enregistrés, avec les tags `devslot` et `ds-<projet>`), ouvrir les ports web sur `http://<label>.localhost:<port>`, et nettoyer les tunnels dont le port a disparu. Voir [Tunnels depuis devslot](/fr/guide/devslot).
- Ouverture en une étape : `tunneler devslot <hôte> <instance>`, ou un lien `tunneler://devslot/<hôte>/<instance>` qui ouvre Tunneler et demande d'abord une confirmation, ouvre les tunnels vers les ports web de l'instance et ouvre le navigateur.

### Distribution
- Installeur Windows (par utilisateur ou pour la machine), paquet Debian, zip portable, tar.gz et une AppImage de l'interface.
- Mode portable : tout reste à côté des exécutables. Le zip Windows est portable ; le tar.gz Linux ne l'est pas, sauf si vous créez le fichier `portable`. Le daemon copie les tunnels d'une copie portable précédente unique voisine de la sienne, et `tunneler status` affiche le fichier de configuration utilisé.
- `tunneler purge` retire toutes les traces laissées sur la machine.
- Mises à jour signées depuis les versions GitHub, appliquées automatiquement avec l'installeur Windows et proposées par un lien pour les autres formes. La vérification a lieu à l'ouverture de la fenêtre, ou à la demande depuis les paramètres et la palette.
- Mémoire du daemon : environ 17 à 25 Mio. L'interface n'en consomme que fenêtre ouverte.

## Prochainement

- Ce que la section [À tester](#a-tester) fera ressortir.

## À tester

L'essentiel du code tourne dans des tests automatiques contre un serveur SSH en processus. Ce qui manque, ce sont de vraies machines. Si vous essayez l'un de ces points, un ticket avec le résultat, bon ou mauvais, aide.

### Windows
- **Installeur.** Installer pour l'utilisateur courant puis pour tous les utilisateurs, avec et sans raccourcis. Mettre à jour depuis la version précédente (les tunnels doivent revenir). Désinstaller avec et sans la case « Supprimer aussi mes tunnels, réglages et passphrases » : il ne doit rien rester.
- **Tout ce que fait le daemon sous Windows.** Il compile et passe le lint, mais tous les chemins n'ont pas été exécutés : le named pipe, la zone de notification, le lancement à l'ouverture de session, le gestionnaire d'identifiants, l'agent OpenSSH et Pageant, `ProxyCommand` et `Match exec`, la détection des clients VNC et RDP, le lancement détaché.
- **Notifications.** Un toast doit s'afficher au nom de Tunneler sans raccourci du menu Démarrer.
- **Sons de veille.** Tous les écouter, y compris avec le son « Notification » réglé sur « (Aucun) » dans le jeu de sons de Windows. Vérifier qu'aucun ne se coupe brusquement (chacun s'éteint, puis 150 ms de silence) et que le curseur de volume n'est ni trop faible à 5 % ni trop fort à 100 %.
- **Démarrage du serveur X.** Le démarrage de VcXsrv depuis la fenêtre « Me demander » et depuis le panneau X11 a été vérifié : les fenêtres qui lui sont transférées s'affichent. Reste à essayer : Xming, les valeurs « Le démarrer » et « Ne pas le démarrer », et un lanceur X11 qui dessine via le serveur démarré.
- **Lecteur d'écran et affichage.** NVDA avec l'interface, le mode contraste élevé de Windows, et un zoom à 200 % sans défilement horizontal.

### Linux
- **Paquet Debian.** Installer, lancer, mettre à jour et retirer le `.deb`.
- **Mise à jour du tar.gz sous mise.** Installer une version qui contient encore `portable`, ajouter des tunnels, puis `mise upgrade` vers la nouvelle : le daemon doit copier `tunnels.toml` et `state.toml` dans les dossiers de l'utilisateur et le journaliser, et l'ancien dossier doit rester intact. Vérifier ensuite que `mise upgrade --no-prune` garde l'ancienne version.
- **Sons et notifications.** Les sons de veille sur un vrai bureau avec PipeWire (`pw-play -`), et avec PulseAudio seul (`paplay`, jamais essayé). Les notifications D-Bus.
- **Zone de notification sous KDE.**
- **Entrée du menu des applications de l'archive.** Vérifiée sous KDE, installée par la ligne `postinstall` de mise de la page [Premiers pas](/fr/guide/getting-started#linux) : l'entrée, son icône et le regroupement de la fenêtre sous celle-ci. Reste à essayer : GNOME, et un `mise upgrade` depuis une version qui a déjà l'entrée (la ligne `postinstall` doit la pointer sur le nouveau dossier).
- **Lecteur d'écran.** Orca, écouté par une personne. Le passage au clavier et l'arbre d'accessibilité ont été vérifiés.
- **Mémoire.** L'interface pèse environ 410 Mio PSS sous Xvfb en rendu logiciel. Un vrai bureau avec GPU devrait en prendre moins : une mesure le confirmerait.

### Les deux systèmes
- **Découverte devslot.** Vérifié contre un vrai hôte : découvrir, démarrer des tunnels temporaires (y compris sur un port déjà utilisé localement), ouvrir un port web dans le navigateur. Reste à vérifier : les tunnels enregistrés, arrêter une instance puis redécouvrir (les tunnels périmés doivent partir ou faire l'objet d'une question), un hôte où `ds` n'est pas dans le `PATH` non interactif (avec et sans `devslot_path`), et une instance avec un port imposé.
- **Lien devslot.** Vérifié sous Linux avec `tunneler desktop-entry install` : `tunneler devslot` contre un vrai hôte (un checkout principal et un worktree, lancés deux fois), et un lien ouvert avec `xdg-open`. Reste à vérifier : cliquer sur un lien `tunneler://devslot/...` après l'installeur Windows et le `.deb` : la confirmation doit nommer l'hôte et l'instance. Les invites que Chrome et Firefox affichent avant d'ouvrir une application externe. Quand la fenêtre est déjà ouverte sous Windows, elle doit passer au premier plan. Avec NVDA et Orca : la confirmation, l'état en cours et un échec (lu une seule fois, en commençant par ce qui n'a pas marché). Et `tunneler devslot --no-open`.
- **Export et import.** Les boîtes de dialogue natives de l'onglet « Fichier » sous Windows et sous Linux : annuler, un chemin très long, un fichier impossible à écrire. Un vrai déménagement d'un ordinateur Windows vers un portable Linux : exporter, copier, importer, remplacement des hôtes, tunnels risqués, puis démarrage des tunnels. Une passe au lecteur d'écran sur l'onglet « Fichier » : les filtres, l'aperçu avec ses états et ses risques lus comme du texte, le choix d'alias et le résultat annoncé.
- **X11.** VcXsrv sous Windows et Xorg ou Xwayland sous Linux, contre un vrai `sshd` : un `xeyes` distant doit s'afficher sans `xauth`.
- **Applications installées pendant que Tunneler tourne.** Installer TigerVNC, Remmina ou FreeRDP pendant que Tunneler tourne, puis ouvrir la fenêtre : l'application doit apparaître dans Paramètres › Ouverture sans redémarrage, et un tunnel sur son port doit s'ouvrir avec elle.
- **SOCKS.** `curl --socks5` à travers un tunnel dynamique vers un vrai `sshd`.
- **L'interface telle qu'on la voit.** La veille forcée (menu, barre de sélection, clic droit sur « Tous les tunnels », entrée de la zone de notification), les tiroirs, la fenêtre de réglages et ses onglets, la palette de commandes, le panneau replié, le renommage et la suppression de tag, le formulaire « Ouverture » et la section « Applications », les badges de veille, les infobulles, le clic droit et la sélection multiple.
- **Fermer la fenêtre des réglages avec des changements non enregistrés.** Échap, un clic à l'extérieur et le bouton de fermeture doivent chacun demander « Abandonner vos modifications ? ». « Continuer la modification » doit remettre le focus sur le champ en cours d'édition.
- **Notifications.** Avec Orca (Linux) ou NVDA (Windows) : les notifications sont annoncées, y compris quand un tiroir ou une boîte de dialogue est ouvert, poliment pour les avertissements et de façon assertive pour les erreurs. F8 doit atteindre les notifications puis ramener là où on était, et Échap doit fermer celle qui a le focus. À la taille minimale de la fenêtre, une notification qui reste jusqu'à sa fermeture ne doit pas masquer entièrement le contrôle ayant le focus d'un tiroir ouvert.
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

### Découverte devslot
- Le lien `tunneler://` n'est pas déclaré par le zip portable Windows, l'AppImage ni macOS : utilisez la commande. Les navigateurs demandent en général une confirmation avant d'ouvrir une application externe, et un terminal ne rend pas toujours le lien cliquable.
- L'alias d'un lien vient de la configuration devslot du serveur. Un ordinateur qui connaît le serveur sous un autre alias obtient des liens qui échouent avec un alias inconnu : tapez `tunneler devslot` avec son propre alias.
- `tunneler devslot` ne tunnele que les ports web, et pas ceux inférieurs à 1024. Les services comme une base de données, et les tunnels enregistrés périmés qui demandent une décision, sont laissés au tiroir « Importer et exporter ».
- `devslot_path` et `devslot_persist` d'un hôte s'éditent à la main dans `tunnels.toml` : la fenêtre ne les écrit pas.
- Quand le port local diffère du port distant, les liens que l'application construit (URL absolues dans une page) pointent toujours vers le port distant et ne fonctionnent pas.

### Export et import
- Il n'y a pas de mode remplacement : un import ne modifie ni ne supprime jamais un tunnel. Pour en changer un, modifiez-le dans la fenêtre.
- Un tunnel dont le port est déjà utilisé est ignoré, pas déplacé sur un autre port. Changez le port dans le fichier, ou ajoutez le tunnel à la main.
- Les réglages, les applications, les lanceurs X11 et les réglages par hôte ne sont pas transportés. Recopiez-les à la main depuis `tunnels.toml`.
- Deux tunnels qui écoutent sur `127.0.0.1:P` et sur `0.0.0.0:P` se heurtent au niveau du système, mais l'import ne le détecte pas : le second échoue à son démarrage.
- La ligne de commande n'importe que la sélection par défaut : un tunnel risqué s'ajoute depuis la fenêtre.

### Interface et ligne de commande
- L'import depuis un script collé ne reprend pas le `$env:DISPLAY` dans `x11_display`, et la ligne de commande n'a pas d'import depuis `ssh_config` ou depuis des scripts (`tunneler import` lit un fichier).
- Les heures de santé en ligne de commande sont en UTC.
- Un `tunnels.toml` en fins de ligne CRLF n'est pas garanti identique après réécriture en dehors des lignes que Tunneler modifie.
- Les bordures de certains contrôles de la bibliothèque de composants sont sous 3:1 de contraste (WCAG 1.4.11). C'est accepté pour l'instant.
- WebKitGTK expose le bouton du menu d'actions comme une liste déroulante (« combo box »).
- Les choix « English » et « Français » du réglage de langue ne portent pas leur langue pour les lecteurs d'écran : la bibliothèque de composants ne permet pas encore de poser `lang` sur une option.
- Les toasts Windows portent le nom Tunneler mais pas d'icône.
- Les binaires et installeurs ne sont pas signés : Windows SmartScreen avertit au premier lancement.
- macOS n'est pas maintenu : ni paquet, ni CI, ni zone de notification.
