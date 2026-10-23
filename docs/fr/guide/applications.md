# Ouvrir un tunnel avec une application

Un tunnel local peut s'ouvrir dans votre navigateur ou avec un autre programme : un client VNC, un client RDP, un client SQL. Cette page explique comment Tunneler en choisit un, quels programmes il connaît et comment ajouter les vôtres.

Seuls les tunnels locaux peuvent être ouverts. Un tunnel distant écoute sur le serveur, et un tunnel dynamique est un proxy.

## Avec quoi un tunnel s'ouvre

Quand vous ouvrez un tunnel, Tunneler prend le premier de ces choix qui s'applique :

1. Ce que vous avez demandé : « Ouvrir avec… » dans un menu, une commande de la palette, ou `--with` en ligne de commande.
2. L'application enregistrée dans le formulaire du tunnel, sous « Ouverture » › « Application ».
3. La première application disponible dont les ports suggérés contiennent le **port cible** du tunnel.
4. Le navigateur.

L'étape 3 regarde le port cible, pas le port d'écoute. Un tunnel qui écoute sur `5999` et atteint le port `5901` de `nova` est un tunnel VNC : un client VNC lui est suggéré. Vos applications passent avant celles qui sont intégrées. Les applications intégrées suivent l'ordre des tableaux ci-dessous.

Les applications intégrées suggèrent ces ports :

| Ports | Suggéré pour |
|---|---|
| 5900 à 5999 | VNC |
| 3389 | RDP |

Le programme est lancé avec `{host}` et `{port}` qui désignent le côté **écoute** du tunnel. `{port}` est le port local, `5999` dans l'exemple. `{host}` vaut `localhost`, ou `127.0.0.1` quand vous avez choisi l'adresse IP sous « Ouverture » dans les paramètres ou dans le formulaire du tunnel. Un tunnel qui écoute sur une adresse IP précise utilise cette adresse.

Dans le menu du tunnel, l'action d'ouverture dit ce qui va se passer : « Ouvrir dans le navigateur » ou « Ouvrir avec TigerVNC ». « Ouvrir avec… » liste les autres, et la palette de commandes a une commande par application.

## Applications intégrées

Tunneler cherche ces programmes une seule fois, au démarrage du daemon. Après en avoir installé un, quittez Tunneler et relancez-le. Ceux qu'il trouve apparaissent dans « Paramètres » › « Ouverture » › « Applications » › « Détectées sur ce poste ».

### Windows

| Id | Application | Trouvée | Arguments | Ports |
|---|---|---|---|---|
| `mstsc` | Remote Desktop (mstsc) | toujours présente | `/v:{host}:{port}` | 3389 |
| `tigervnc` | TigerVNC | `vncviewer` dans le `PATH`, sinon `C:\Program Files\TigerVNC\vncviewer.exe`, sinon `C:\Program Files (x86)\TigerVNC\vncviewer.exe` | `{host}::{port}` | 5900 à 5999 |
| `realvnc` | VNC Viewer (RealVNC) | `C:\Program Files\RealVNC\VNC Viewer\vncviewer.exe`, sinon l'équivalent dans `Program Files (x86)` | `{host}::{port}` | 5900 à 5999 |

### Linux

| Id | Application | Trouvée | Arguments | Ports |
|---|---|---|---|---|
| `remmina-vnc` | Remmina (VNC) | `remmina` dans le `PATH` | `-c`, `vnc://{host}:{port}` | 5900 à 5999 |
| `remmina-rdp` | Remmina (RDP) | `remmina` dans le `PATH` | `-c`, `rdp://{host}:{port}` | 3389 |
| `tigervnc` | TigerVNC | `vncviewer` dans le `PATH` | `{host}::{port}` | 5900 à 5999 |
| `freerdp` | FreeRDP | `xfreerdp3`, sinon `xfreerdp`, dans le `PATH` | `/v:{host}:{port}` | 3389 |

Il n'y a pas de client SQL intégré : les arguments dépendent du client et du pilote. [Ajoutez-le vous-même](#un-client-sql).

## Ajouter une application

Ouvrez « Paramètres » (la roue dentée de l'en-tête, ou la touche `,`) et allez sur l'onglet « Ouverture ». Il contient la section « Applications ». La liste « Détectées sur ce poste » montre ce que Tunneler a trouvé. En dessous, « Ajoutées » liste les vôtres.

Si le programme voulu est installé là où Tunneler ne regarde pas, comme une copie portable, utilisez « Ajouter une application » :

| Champ | À saisir |
|---|---|
| « Nom » | Le libellé affiché dans les menus. |
| « Identifiant » | Des minuscules, des chiffres et des tirets. Il est rempli d'après le nom. Les tunnels désignent l'application par cet identifiant : il n'est plus modifiable après l'ajout. `browser` est réservé. |
| « Commande » | Le programme à lancer, sans ses arguments. Donnez le chemin complet quand il n'est pas dans le `PATH`. |
| « Arguments » | Un argument par ligne. |
| « Ports suggérés » | Optionnel, séparés par des virgules. Un tunnel dont le port cible est dans la liste s'ouvre par défaut avec cette application. |

Comment fonctionnent les arguments :

- `{host}`, `{port}` et `{url}` sont remplacés. `{url}` est l'adresse que recevrait le navigateur, comme `http://localhost:5999/`. Aucun autre `{…}` n'est accepté.
- Aucun shell ne lance la commande. Les guillemets et les espaces ne veulent rien dire pour Tunneler : chaque ligne est transmise telle quelle au programme, et les espaces d'un chemin n'ont pas besoin de guillemets.
- Les variables comme `%USERPROFILE%` ou `~` ne sont pas développées.

Pour modifier une de vos applications, utilisez « Modifier » à côté d'elle, puis « Mettre à jour ». « Supprimer » la retire. Les tunnels réglés pour s'ouvrir avec elle ne peuvent plus s'ouvrir tant que vous n'en choisissez pas une autre.

Rien n'est enregistré au fil de l'eau. Ajouter, modifier et supprimer ne changent que la liste de la fenêtre des paramètres, et les paramètres sont enregistrés d'un coup avec « Enregistrer » en bas. « Annuler » demande confirmation avant d'abandonner vos changements. Le formulaire d'une application qui contient encore une saisie bloque « Enregistrer » tant que vous n'avez pas ajouté l'application ou annulé le formulaire.

### Remplacer une application intégrée

Une application à vous qui a le même identifiant qu'une application intégrée la remplace : l'intégrée n'est plus proposée, et la vôtre est utilisée partout où elle l'était. Supprimez la vôtre et l'application intégrée revient.

Le plus rapide est le bouton « Personnaliser » à côté d'une application détectée. Il ouvre le formulaire rempli avec les valeurs intégrées et le même identifiant. Changez ce qu'il faut et validez.

Sous la liste des applications détectées, une aide dit : « Installée ailleurs, par exemple une version portable ? Ajoutez-la ci-dessous avec le chemin complet de son programme. » Son lien « Comment ajouter une application » ouvre cette page.

### Exemple : un TigerVNC portable sous Windows

Vous avez téléchargé la version portable du client TigerVNC et l'avez placée dans `C:\Tools\TigerVNC\vncviewer64-1.14.1.exe`. Tunneler ne la trouve pas, donc un tunnel VNC s'ouvre dans le navigateur. Pour en faire le client par défaut des tunnels VNC :

1. Dans « Paramètres » › « Ouverture » › « Applications », cliquez sur « Ajouter une application ».
2. « Nom » : `TigerVNC`.
3. « Identifiant » : `tigervnc`. C'est l'identifiant du TigerVNC intégré, donc le vôtre le remplace. Avec un autre identifiant, les deux existent côte à côte.
4. « Commande » : `C:\Tools\TigerVNC\vncviewer64-1.14.1.exe`.
5. « Arguments » : une seule ligne, `{host}::{port}`. Le double deux-points indique à TigerVNC que le nombre est un port TCP, pas un numéro d'écran.
6. « Ports suggérés » : `5900, 5901, 5902`, ou les ports de vos serveurs VNC.
7. Cliquez sur « Ajouter », puis sur « Enregistrer » dans les paramètres.

Un tunnel de `5999` vers `nova:5901` s'ouvre maintenant avec ce client, qui reçoit `localhost::5999`.

La même application dans `tunnels.toml`. Mettez le chemin Windows entre apostrophes pour que les antislashs restent tels quels :

```toml
[[openers]]
id = "tigervnc"
name = "TigerVNC"
command = 'C:\Tools\TigerVNC\vncviewer64-1.14.1.exe'
args = ["{host}::{port}"]
ports = [5900, 5901, 5902]
```

Tunneler recharge le fichier quand il change.

### Un client SQL

Aucun client SQL n'est intégré : un tunnel de base de données s'ouvre donc dans le navigateur, sauf si vous en ajoutez un. DBeaver peut se connecter depuis la ligne de commande. Ceci ouvre une connexion PostgreSQL vers le tunnel :

```toml
[[openers]]
id = "dbeaver"
name = "DBeaver"
command = "dbeaver"
args = ["-con", "driver=postgresql|host={host}|port={port}"]
ports = [5432]
```

Un tunnel dont le port cible est 5432 s'ouvre alors avec DBeaver. La syntaxe de `-con` appartient à DBeaver : consultez sa documentation pour le pilote que vous utilisez.

## Choisir l'application d'un tunnel

Dans le formulaire du tunnel, la section « Ouverture » a un champ « Application ». Il propose :

- « Automatique », suivi du nom de ce qui serait choisi : c'est le comportement par défaut.
- « Navigateur », pour toujours utiliser le navigateur, même sur un port VNC.
- Toutes les applications disponibles.

Sous les champs, une ligne montre le résultat : « Ouvre » suivi de `http://localhost:3000/` ou « Lance » suivi de la ligne de commande. La même section règle le schéma (`http` ou `https`), le chemin et l'hôte pour le navigateur.

Dans `tunnels.toml`, c'est `open.with` :

```toml
[[tunnels]]
host = "nova"
kind = "local"
listen_port = 5999
target_port = 5901
open = { with = "tigervnc" }
```

Un identifiant qui ne correspond à aucune application n'est pas une erreur au chargement du fichier, puisqu'un programme peut être désinstallé. L'ouverture de ce tunnel échoue, et le message liste ce qui est disponible.

## En ligne de commande

```sh
tunneler open 5999                    # le choix propre au tunnel
tunneler open 5999 --with tigervnc    # une application donnée
tunneler open 5999 --with browser     # le navigateur, quel que soit le port
```

`--with` accepte `browser` ou un identifiant. Avec un identifiant que Tunneler ne connaît pas, `tunneler open` liste les identifiants disponibles et se termine avec le code 9. Le code 8 signifie que le tunnel n'est pas devenu prêt ou que le programme n'a pas pu être lancé. Voyez les [codes de sortie](../reference/cli#codes-de-sortie).
