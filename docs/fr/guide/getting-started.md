# Prise en main

Tunneler garde vos redirections de ports SSH en marche. Un petit daemon tient les sessions SSH et affiche une icône dans la zone de notification. La fenêtre est un programme à part : vous l'ouvrez quand vous en avez besoin et la fermez ensuite, sans arrêter les tunnels.

Il lit votre `~/.ssh/config` et votre `known_hosts`, et il embarque son propre client SSH : vous n'avez pas besoin d'avoir `ssh`.

## Installer

Les paquets sont sur la [page des versions](https://github.com/Sonny93/tunneler/releases). Ils ne sont pas signés : Windows SmartScreen prévient au premier lancement, choisissez « Informations complémentaires » puis « Exécuter quand même ».

### Windows

- **Installeur**, `tunneler-<version>-windows-x86_64-setup.exe`. Il demande si l'installation se fait pour l'utilisateur courant (`%LOCALAPPDATA%\Programs\Tunneler`) ou pour tous les utilisateurs (`Program Files`). Il installe `tunneler-ui.exe` et `tunneler.exe` côte à côte et ajoute une entrée au menu Démarrer.
- **Zip**, `tunneler-<version>-windows-x86_64.zip`. Décompressez-le où vous voulez et lancez `tunneler-ui.exe`, ou `tunneler.exe` pour la ligne de commande. Gardez les deux programmes dans le même dossier. Cette forme est [portable](./updates-and-uninstall#mode-portable) : ses fichiers restent dans ce dossier.

### Linux

- **Debian et Ubuntu**, `tunneler_<version>_amd64.deb`, construit sur Ubuntu 22.04. Il installe `tunneler`, `tunneler-ui` et une entrée de menu, et tire WebKitGTK 4.1 et GTK 3.

  ```sh
  sudo apt install ./tunneler_<version>_amd64.deb
  ```

- **Archive**, `tunneler-<version>-linux-x86_64.tar.gz`. Décompressez-la et lancez `./tunneler-ui`, ou `./tunneler` pour la ligne de commande. Il faut glibc 2.35 ou plus récente, et l'interface demande WebKitGTK 4.1 et GTK 3 :

  ```sh
  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0
  ```

  Cette forme est [portable](./updates-and-uninstall#mode-portable).
- **AppImage**, `tunneler-ui-<version>-x86_64.AppImage`. Utilisez-la quand `libwebkit2gtk-4.1` ne peut pas être installé. Elle ne contient que la fenêtre : placez-la à côté de `tunneler` sous le nom `tunneler-ui`, ou donnez son chemin dans `TUNNELER_UI_BINARY`.

macOS n'a pas de paquet et n'est pas maintenu.

## Premier lancement

Lancez Tunneler depuis le menu Démarrer, le menu des applications, ou avec `tunneler-ui`. Le daemon démarre, une icône apparaît dans la zone de notification et la fenêtre s'ouvre.

Fermez la fenêtre : l'icône reste et les tunnels continuent. Pour rouvrir la fenêtre, choisissez « Ouvrir Tunneler » dans le menu de l'icône ou lancez `tunneler` dans un terminal. Pour tout arrêter, choisissez « Quitter » dans le menu de l'icône ou lancez `tunneler quit`.

Sous GNOME, il n'y a pas de zone de notification par défaut. Installez l'extension « AppIndicator and KStatusNotifierItem Support » pour voir l'icône. Sans elle, lancez `tunneler` pour rouvrir la fenêtre.

## Ajouter un premier tunnel

Supposons que `~/.ssh/config` contienne un hôte `nova`, et qu'une application web écoute sur le port 3000 de ce serveur :

```
Host nova
  HostName 203.0.113.10
  User alice
  IdentityFile ~/.ssh/id_ed25519
```

1. Cliquez sur « Nouveau tunnel ».
2. Dans « Hôte », saisissez ou choisissez `nova`. Le champ propose les alias de votre `ssh_config`.
3. Laissez le type sur « Local (-L) » et mettez `3000` dans « Port d'écoute ». « Port cible » le suit.
4. Cliquez sur « Créer ».

Le tunnel apparaît dans la liste, groupé sous `nova`. La même chose depuis un terminal, sans rien enregistrer :

```sh
tunneler up nova -L 3000
```

## Le démarrer et l'ouvrir

Activez l'interrupteur du tunnel. La première fois, Tunneler peut demander quelque chose :

- **Une passphrase** pour une clé chiffrée, ou un mot de passe. Vous pouvez faire mémoriser la passphrase pour la session ou dans le trousseau du système. Les mots de passe ne sont jamais mémorisés.
- **Une clé d'hôte** qu'il n'a jamais vue. Comparez l'empreinte SHA-256 avec celle que l'administrateur du serveur vous a donnée, puis choisissez « Accepter et enregistrer », « Accepter une fois » ou « Refuser ». Une clé d'hôte qui a changé est refusée, sans moyen de passer outre depuis la fenêtre.

Quand l'état indique « Actif », ouvrez le tunnel : utilisez « Ouvrir dans le navigateur » dans le menu de la ligne, ou lancez :

```sh
tunneler open 3000
```

Votre navigateur ouvre `http://localhost:3000/`. Un tunnel arrêté est démarré d'abord. Pour ouvrir un tunnel avec un autre programme que le navigateur, comme un client VNC ou RDP, voyez [Ouvrir un tunnel avec une application](./applications).

## Trouver de l'aide

Le bouton « Aide » de l'en-tête de la fenêtre, la touche `F1` et la commande « Ouvrir la documentation » de la palette (`Ctrl+K`) ouvrent ce site. Dans les paramètres, la section « Applications » de l'onglet « Ouverture » renvoie vers [Ouvrir un tunnel avec une application](./applications).

## Et ensuite

- [Tunnels](./tunnels) : redirections locales, distantes et dynamiques, tags et veille.
- [Ligne de commande](../reference/cli) : toutes les commandes `tunneler`.
