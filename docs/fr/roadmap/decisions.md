# Décisions de conception

Ces décisions sont tranchées. En changer une impose de mettre cette page à jour en disant pourquoi.

## Décisions

### Client SSH embarqué
Tunneler utilise la bibliothèque `russh` et ne lance jamais le `ssh` du système. Une session par hôte porte tous les tunnels comme autant de canaux : activer ou couper un tunnel ne reconnecte rien. Le comportement est aussi le même sur tous les systèmes, et on garde la main sur les états, les métriques, les invites et X11 sans `xauth`.

### Un daemon et une interface à la demande
Le daemon n'a pas d'interface (zone de notification, SSH, IPC) et reste léger. La fenêtre Tauri est un processus à part qui n'existe que tant qu'elle est ouverte : fermée, elle ne coûte aucune mémoire. La webview seule prend 100 à 200 Mio.

### Analyseur de `ssh_config` maison
Les bibliothèques Rust existantes n'ont ni `Match` ni expansion des jetons, ou sont trop limitées. L'analyseur est écrit dans le projet et comparé à `ssh -G` : OpenSSH sert de référence.

### Analyseur de `known_hosts` maison
Celui de `russh` n'a ni jokers ni négation, ni `@cert-authority` ni `@revoked`, son chemin est figé et il écrit les entrées en clair. Celui du projet gère tout cela, entrées hachées comprises.

### Tauri, React et `@minimalstuff/ui` pour l'interface
Tauri 2 avec React 19, TypeScript, Vite et UnoCSS, sur la bibliothèque de composants de l'auteur, dont la base ARIA est solide. Le coût mémoire de la webview n'est payé que fenêtre ouverte.

### Zone de notification avec `tray-icon`
Win32 natif sous Windows. Sous Linux, le protocole StatusNotifierItem par D-Bus (`ksni`) évite toute dépendance à GTK dans le daemon. Pas de zone de notification sous macOS : sa boucle d'événements exigerait le thread principal.

### IPC local en JSON lines
`interprocess` fournit un socket Unix ou un named pipe Windows, limité à l'utilisateur courant. L'interface et la CLI parlent le même protocole.

### Configuration en TOML éditable à la main
`tunnels.toml` vit dans le dossier de config de l'utilisateur, ou à côté des exécutables en mode portable. On peut l'éditer à la main, le comparer avec `diff`, et il est rechargé à chaud. Une version installée ne range jamais de données dans son dossier d'installation : les mises à jour le remplacent, la désinstallation le supprime et `Program Files` n'est pas inscriptible. Une version portable garde tout dans son dossier.

### Des tags, pas des groupes
Un tunnel peut appartenir à plusieurs ensembles (`dev`, `mail`, `adonis`). Activer un tag active ses tunnels.

### Le daemon est l'instance unique
Lier le socket IPC fait office de verrou. Un second lancement devient un client qui demande au daemon d'afficher l'interface : aucun plugin d'instance unique n'est nécessaire.

### Outillage du front
oxlint et oxfmt via les préréglages partagés de `@minimalstuff/tooling`, plus le plugin `jsx-a11y` d'oxlint, absent du préréglage React. `rustfmt` et `clippy` pour Rust. C'est rapide et identique aux autres projets de l'auteur.

### L'accessibilité est une exigence
L'interface vise WCAG 2.2 AA et la CLI doit fonctionner sans couleur et sans terminal. Une fonctionnalité n'est pas terminée tant qu'elle ne s'utilise pas au clavier seul et au lecteur d'écran, ni sans dépendre de la couleur. Voir les [règles d'accessibilité](/fr/contributing/accessibility).

### Les tags sont le seul nom
Un tunnel n'a pas de nom. Un nom répétait l'adresse (`3333 → nova:3333` suffit à identifier un tunnel) et ajoutait un champ à garder unique. Un tunnel s'affiche par sa route et se désigne en ligne de commande par port, par `hôte:port` ou par tag. C'est pourquoi un tag ne peut pas être fait uniquement de chiffres ni contenir `:` : on le confondrait avec un port ou un `hôte:port`.

### Veille : écouter en local, ouvrir SSH à la demande
Un tunnel en veille garde son port local et met les connexions entrantes en attente. La première ouvre la session SSH, puis les connexions en attente sont branchées sur des canaux. Aucune session SSH ne tourne tant que personne ne se connecte. Le daemon possède déjà l'écoute, il n'y a donc pas de processus de plus. La veille est facultative, avec un réglage général et une surcharge par tunnel, et ne vaut que pour les tunnels locaux et dynamiques. Après `standby_idle_s` sans connexion (5 minutes par défaut), le tunnel lâche la session et retourne en veille.

Quand la session ne s'ouvre pas, les connexions en attente sont fermées, une notification part et le tunnel retourne en veille avec la raison. Il ne réessaie jamais en boucle : la connexion suivante retente. Le signal de réveil est une notification, un son ou les deux, et c'est le daemon qui joue le son, pour qu'il reste audible quand les notifications du système sont coupées. Les sons sont synthétisés par le daemon : un son système réglé sur « aucun » répond quand même « succès » sans rien jouer, et le daemon ne peut pas le détecter.

### L'ouverture d'un tunnel revient au daemon
Le daemon implémente l'ouverture (navigateur ou application externe) une seule fois, pour l'interface, la CLI et la palette. Les commandes viennent de la config de l'utilisateur, jamais des paramètres IPC, et sont lancées sans shell.

### Distribution
Un zip et un tar.gz portables, un installeur NSIS sous Windows, un `.deb` sous Linux, une commande `purge` pour nettoyer, et des mises à jour signées par les versions GitHub. Le bundler Tauri produit NSIS et `.deb` sans autre outil. NSIS plutôt que MSI : installation par utilisateur sans droits d'administrateur, choix utilisateur ou machine natif, hooks d'installation. Le nettoyage vit dans le binaire, en une seule implémentation testée qui connaît les chemins et le trousseau, et sert au désinstalleur, à la version portable et à Linux. Seul l'installeur Windows se met à jour tout seul. Les autres formes sont prévenues qu'une nouvelle version existe.

## Fermeture

Les tunnels vivent dans le daemon : ils ne peuvent pas lui survivre.

- **Fermer la fenêtre** ne termine que l'interface. Le daemon et les tunnels continuent, sans confirmation.
- **Quitter Tunneler** depuis la zone de notification ou l'interface, avec des tunnels actifs, demande confirmation : tout arrêter et quitter, ou annuler.

## Écarté

### Laisser les tunnels tourner après avoir quitté
Les tunnels vivent dans le daemon : quitter le daemon les termine. Une option « quitter en laissant tourner » ne peut pas exister avec cette architecture.

### Une sonde de santé périodique
Les services qui se défendent contre la force brute (Xvnc, fail2ban, `MaxStartups` de sshd) comptent chaque connexion non authentifiée comme un échec. Une sonde toutes les quelques secondes finit par faire bannir 127.0.0.1. La vivacité du lien vient du keepalive SSH. Une cible n'est testée que sur demande de l'utilisateur, avec une seule connexion refermée aussitôt. Une connexion refusée marque aussi la cible injoignable, sans trafic en plus.

### La veille pour les tunnels distants
Un tunnel distant écoute côté serveur : tenir le port exige une session ouverte. C'est le contraire de la veille.

### Se reconnecter depuis la veille
Quand la session d'un tunnel en veille réveillé est perdue, le tunnel retourne directement en veille. Ses canaux sont morts avec la session, il n'y a donc plus de connexion à préserver, et une reconnexion automatique ne ferait que rouvrir une session que personne n'utilise.

### La veille pour les tunnels temporaires
Un tunnel temporaire est une demande explicite et immédiate : il démarre actif.

### Un nom pour les tunnels
Voir « Les tags sont le seul nom » plus haut. Un `name` resté dans un ancien `tunnels.toml` est ignoré à la lecture et disparaît à la prochaine écriture.

### Un shell pour lancer les applications
Les applications sont lancées à partir d'une liste d'arguments, où seuls `{host}`, `{port}` et `{url}` sont remplacés. Avec un shell, toute valeur qui finit dans une commande deviendrait un risque d'injection.

### Ouvrir un nom d'hôte distant
L'ouverture d'un tunnel utilise `localhost`, l'adresse de boucle, ou l'IP exacte sur laquelle le tunnel écoute, jamais un nom d'hôte distant : il contournerait le tunnel.

### Un préréglage de client SQL
Les arguments changent d'un pilote de base de données à l'autre, il n'y a donc rien de raisonnable à préconfigurer. Ajoutez votre propre application avec les ports pour lesquels elle doit être proposée.

### Passer outre une clé d'hôte modifiée en un clic
Une clé d'hôte modifiée ou révoquée est un échec définitif avec une alerte explicite. Il n'y a pas de bouton « se connecter quand même ».

### Mettre à jour l'AppImage sous Linux
L'updater de Tauri ne sait mettre à jour que l'AppImage, qui ne contient pas le daemon. L'interface et le daemon se retrouveraient dans des versions différentes. Les formes portable, `.deb`, tar.gz et AppImage affichent seulement un bandeau avec un lien.

### La zone de notification comme seule voie
Tout ce que fait la zone de notification se fait aussi depuis l'interface et la ligne de commande.
