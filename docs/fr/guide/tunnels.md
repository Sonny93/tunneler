# Tunnels

Un tunnel est une redirection de port à travers un hôte de votre `ssh_config`. Les tunnels n'ont pas de nom : la liste montre où chacun écoute et ce qu'il atteint, par exemple `3333 → nova:3333`. Les tunnels qui passent par le même hôte partagent une seule session SSH.

## Types

### Local, `-L`

Écoute sur votre ordinateur et se connecte à la cible depuis le serveur. Servez-vous-en pour atteindre un service que seul le serveur voit : une base de données, une page d'administration, un bureau VNC.

`3333 → nova:3333` écoute sur `127.0.0.1:3333` chez vous et atteint le port 3333 de `nova`. Pour atteindre une autre machine à travers `nova`, renseignez « Hôte cible » (par exemple `db.internal`) et « Port cible ». La liste affiche alors `3333 → db.internal:5432 via nova`.

```sh
tunneler up nova -L 5000:db.internal:5432
```

### Distant, `-R`

Écoute sur le serveur et se connecte à la cible depuis votre ordinateur. Servez-vous-en pour que le serveur, ou quelqu'un dessus, atteigne un service qui tourne chez vous.

`nova:9000 → 127.0.0.1:3000` écoute sur le port 9000 de `nova` et atteint le port 3000 chez vous. Par défaut, Tunneler demande au serveur d'écouter sur `localhost` ; changez « Adresse d'écoute » pour autre chose. Le réglage `GatewayPorts` du serveur a le dernier mot.

```sh
tunneler up nova -R 9000:localhost:3000
```

### Dynamique, `-D`

Écoute sur votre ordinateur comme un proxy SOCKS5. Chaque client dit où il veut aller et la connexion part du serveur. Servez-vous-en pour naviguer ou lancer un outil comme si vous étiez sur le réseau du serveur. Un tunnel dynamique n'a pas de cible.

```sh
tunneler up nova -D 1080
curl --socks5-hostname 127.0.0.1:1080 http://intranet.example/
```

## Tags

Un tag regroupe des tunnels. Donnez à un tunnel un ou plusieurs tags, séparés par des virgules, dans son formulaire. Un tag ne contient ni espace, ni virgule, ni deux-points, et n'est pas composé uniquement de chiffres : la ligne de commande y verrait un port, ou une paire `hôte:port`.

Le panneau latéral liste les tags avec le nombre de tunnels actifs. L'interrupteur d'un tag démarre ou arrête tout le groupe. Depuis un terminal :

```sh
tunneler up dev
tunneler down dev
```

Pour démarrer un tag au lancement de Tunneler, ajoutez-le à « Tags démarrés au lancement » dans les paramètres. Le menu d'un tag, dans le panneau latéral, permet de le renommer ou de le supprimer. Supprimer un tag ne supprime jamais ses tunnels.

## Veille

Un tunnel en veille garde son port local mais n'ouvre aucune session SSH. La session s'ouvre quand quelque chose se connecte au port, et la connexion attend que le tunnel soit prêt. Quand plus aucune connexion n'est ouverte depuis un moment, le tunnel retourne en veille et la session se ferme.

C'est adapté aux tunnels que vous gardez configurés mais que vous utilisez de temps en temps : pas de connexion inutile vers le serveur, pas de trafic keepalive, et une passphrase demandée seulement quand vous en avez besoin. Si la connexion échoue, les connexions en attente sont fermées, vous recevez une notification et le tunnel retourne en veille. La connexion suivante réessaie.

- La veille s'applique aux tunnels locaux et dynamiques. Un tunnel distant écoute sur le serveur : il a besoin d'une session.
- Activez-la pour tous les tunnels avec « Mettre les tunnels en veille » dans les paramètres, ou tunnel par tunnel avec le choix « Veille » du formulaire : « Réglage général », « Oui » ou « Non ». Un changement s'applique au prochain démarrage du tunnel.
- « Retour en veille après (secondes) » règle le temps de calme. La valeur par défaut est 300.
- « Signal à la sortie de veille » choisit ce qui se passe quand un tunnel se réveille ou n'y arrive pas : rien, une notification silencieuse, un son, ou les deux. Le son peut être un carillon, un marimba, une corde pincée ou le son du système, chacun avec un bouton d'écoute.
- Un tunnel réveillé peut être remis en veille à la main, un par un, par tag ou tous à la fois : « Mettre en veille » dans les menus, ou « Mettre en veille les tunnels réveillés » dans le menu de l'icône.
- Les tunnels temporaires n'attendent jamais : c'est une demande explicite, ils démarrent actifs.

## Tunnels temporaires

Un tunnel temporaire se démarre depuis la ligne de commande avec la syntaxe de `ssh`. Il n'est jamais enregistré dans `tunnels.toml` et disparaît quand vous l'arrêtez ou quand le daemon quitte.

```sh
tunneler up nova -L 5000
tunneler up nova -D 1080
tunneler down 5000
```

Dans la fenêtre, il porte le badge « Temporaire ». Pour en garder un, utilisez « Enregistrer le tunnel » dans son menu. Vous pouvez aussi en créer un depuis la fenêtre avec « Tunnel temporaire… » dans le menu à côté de « Nouveau tunnel ».

Si un tunnel enregistré fait déjà la même redirection, `tunneler up` démarre celui-là.

## Santé

L'état d'un tunnel dit si le lien avec le serveur fonctionne. Il ne dit pas si le service derrière répond. Tunneler l'apprend de façon passive : quand le serveur refuse une connexion vers la cible, le tunnel est marqué « Cible injoignable » avec la raison.

Tunneler ne sonde jamais la cible à intervalle régulier. Certains services, comme Xvnc, fail2ban ou le `MaxStartups` de `sshd`, comptent chaque connexion fermée sans authentification comme un échec, et une sonde finirait par faire bloquer votre adresse.

Pour tester une fois, à la demande, utilisez « Tester la cible » dans les détails du tunnel, ou :

```sh
tunneler check 3333
```

Le test ouvre une connexion vers la cible et la referme aussitôt. Il se termine avec le code 7 quand la cible ne répond pas. Il fonctionne sur un tunnel local ou distant en marche, pas sur un tunnel dynamique ni sur un tunnel en veille.

## Fermer la fenêtre et quitter

Fermer la fenêtre ferme le programme qui la dessine. Le daemon et les tunnels continuent.

Pour arrêter, choisissez « Quitter » dans le menu de l'icône, « Quitter Tunneler » dans la fenêtre, ou lancez `tunneler quit`. Quand des tunnels sont actifs, il demande d'abord confirmation et propose « Tout arrêter et quitter ». Les tunnels tournent dans le daemon : ils s'arrêtent avec lui. Sauf si vous avez désactivé « Rétablir les tunnels actifs au lancement », ils redémarrent au prochain lancement.
