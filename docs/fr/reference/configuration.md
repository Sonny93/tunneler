# Configuration

Vos tunnels et vos paramètres vivent dans un seul fichier TOML, `tunnels.toml`. La fenêtre le modifie, et vous pouvez le modifier à la main : Tunneler le recharge quand il change. Quand la fenêtre réécrit le fichier, elle garde vos commentaires et votre mise en forme. Les écritures sont atomiques.

Un fichier avec une erreur de syntaxe, une clé inconnue ou une règle non respectée est refusé, et Tunneler dit ce qui ne va pas. Toutes les clés ci-dessous sont optionnelles, sauf celles dont un tunnel a besoin.

## Emplacement des fichiers

| Fichier | Linux | Windows | Portable |
|---|---|---|---|
| `tunnels.toml` | `~/.config/tunneler/` | `%APPDATA%\tunneler\` | à côté des programmes |
| `state.toml` | `~/.local/state/tunneler/` | `%LOCALAPPDATA%\tunneler\` | `data/` |
| Journaux | `~/.local/share/tunneler/logs/` | `%LOCALAPPDATA%\tunneler\logs\` | `data/logs/` |
| Données web de la fenêtre | `~/.local/share/com.minimalstuff.tunneler/` | `%LOCALAPPDATA%\com.minimalstuff.tunneler\` | `data/webview/` |

Sous Linux, les trois premiers respectent `XDG_CONFIG_HOME`, `XDG_STATE_HOME` et `XDG_DATA_HOME`. Les journaux sont des fichiers quotidiens, et le daemon en garde 7. Réglez `TUNNELER_LOG` sur un filtre comme `debug` pour plus de détails. « Ouvrir le dossier de configuration », dans les paramètres, ouvre le dossier de `tunnels.toml`.

Le mode portable est actif tant qu'un fichier vide nommé `portable` se trouve à côté des programmes. Voyez [Mises à jour et désinstallation](../guide/updates-and-uninstall#mode-portable).

### Choisir d'autres fichiers

| Option | Variable d'environnement | Ce qu'elle règle |
|---|---|---|
| `--config <CHEMIN>` | `TUNNELER_CONFIG` | Le fichier des tunnels. |
| `--state <CHEMIN>` | `TUNNELER_STATE` | Le fichier d'état. |
| `--ssh-config <CHEMIN>` | `TUNNELER_SSH_CONFIG` | Le `ssh_config` à lire à la place de `~/.ssh/config` et de celui du système, comme `ssh -F`. |
| `--socket <CHEMIN>` | `TUNNELER_SOCKET` | Le socket, ou le tube nommé sous Windows, sur lequel le daemon écoute. |

Elles l'emportent sur les emplacements ci-dessus, en mode portable ou non. Elles valent pour toutes les commandes, et un daemon lancé par une commande reçoit les mêmes.

## Exemple

```toml
version = 1                         # la version du format ; 1 si absente

[settings]
default_bind_address = "127.0.0.1"  # les tunnels locaux et dynamiques écoutent ici
default_remote_bind_address = "localhost"  # les tunnels distants écoutent ici, sur le serveur
keepalive_interval_s = 15           # 0 désactive le keepalive
keepalive_max = 3                   # sondes sans réponse avant de perdre la session
session_idle_grace_s = 30           # une session inutilisée reste ouverte ce temps
reconnect_initial_delay_ms = 1000
reconnect_max_delay_ms = 60000
restore_on_launch = true            # relance les tunnels qui tournaient à la dernière sortie
start_on_launch_tags = ["dev"]      # tags démarrés au lancement du daemon
launch_at_login = false
use_os_keyring = false              # autorise la mémorisation des passphrases dans le trousseau du système
x11_display = ""                    # vide : $DISPLAY, ou 127.0.0.1:0.0 sous Windows
x11_server_start = "ask"            # ask | always | never
open_host = "localhost"             # localhost | ip
open_default = "suggested"          # suggested | browser
standby = false                     # les tunnels locaux et dynamiques sont en veille par défaut
standby_idle_s = 300                # retour en veille après ce délai sans connexion
standby_notify = "visual"           # none | visual | sound | both
standby_sound = "pluck"             # system | chime | marimba | pluck
standby_volume = 50                 # 5 à 100 (%), pour chime, marimba et pluck
language = "system"                 # system | en | fr
check_for_updates = true            # cherche une version plus récente à l'ouverture de la fenêtre

[[tunnels]]
id = "01JA0000000000000000000000"   # un ULID ; Tunneler l'ajoute s'il manque
host = "nova"                       # un alias de ~/.ssh/config
kind = "local"                      # local | remote | dynamic
listen_port = 3333
# listen_address = "0.0.0.0"        # au lieu de l'adresse d'écoute par défaut
# target_host = "127.0.0.1"         # où le serveur se connecte ; 127.0.0.1 par défaut
# target_port = 3333                # par défaut : listen_port
tags = ["dev", "web"]
# standby = true                    # au lieu de settings.standby
# open = { scheme = "https", path = "/admin", host = "ip", with = "tigervnc" }

[[tunnels]]
host = "nova"
kind = "local"
listen_port = 5432
target_host = "db.internal"         # atteint depuis nova
tags = ["dev"]

[[tunnels]]
host = "nova"
kind = "dynamic"                    # un proxy SOCKS5, pas de cible
listen_port = 1080

[[openers]]                         # applications que vous avez ajoutées ; les intégrées n'y figurent pas
id = "dbeaver"                      # minuscules, chiffres, tirets ; « browser » est réservé
name = "DBeaver"
command = "dbeaver"
args = ["-con", "driver=postgresql|host={host}|port={port}"]
ports = [5432]                      # ports cibles pour lesquels l'application est suggérée

[hosts.nova]                        # réglages propres à l'application, par alias ssh_config
x11_display = "198.51.100.135:0.0"

[[x11_launchers]]
host = "nova"
name = "VS Code"
command = "code --wait"
```

## `[settings]`

| Clé | Type | Défaut | Signification |
|---|---|---|---|
| `default_bind_address` | chaîne | `"127.0.0.1"` | Adresse d'écoute des tunnels locaux et dynamiques. Non vide. |
| `default_remote_bind_address` | chaîne | `"localhost"` | Adresse sur laquelle les tunnels distants demandent au serveur d'écouter. Non vide. |
| `keepalive_interval_s` | entier | `15` | Secondes entre deux sondes keepalive. `0` désactive le keepalive. Un `ServerAliveInterval` du `ssh_config` l'emporte. |
| `keepalive_max` | entier | `3` | Sondes sans réponse avant que la session soit considérée perdue. Au moins 1 tant que le keepalive est actif. Un `ServerAliveCountMax` du `ssh_config` l'emporte. |
| `session_idle_grace_s` | entier | `30` | Secondes pendant lesquelles une session SSH reste ouverte après l'arrêt de son dernier tunnel. |
| `reconnect_initial_delay_ms` | entier | `1000` | Première attente avant de se reconnecter. Supérieure à 0 et pas au-dessus du maximum. |
| `reconnect_max_delay_ms` | entier | `60000` | Borne haute de l'attente, qui croît de façon exponentielle avec un peu de gigue. |
| `restore_on_launch` | booléen | `true` | Relance les tunnels qui tournaient à la dernière sortie. |
| `start_on_launch_tags` | tableau de chaînes | `[]` | Tags dont les tunnels démarrent au lancement du daemon. |
| `launch_at_login` | booléen | `false` | Démarre le daemon à l'ouverture de session. |
| `use_os_keyring` | booléen | `false` | Propose de mémoriser les passphrases de clés dans le trousseau du système (gestionnaire d'identification sous Windows, Secret Service sous Linux). Désactivé, une passphrase mémorisée dure jusqu'à la fermeture de Tunneler. |
| `x11_display` | chaîne | `""` | Display X local. Vide signifie `$DISPLAY`, ou `127.0.0.1:0.0` sous Windows. |
| `x11_server_start` | `"ask"`, `"always"` ou `"never"` | `"ask"` | Que faire quand aucun serveur X ne répond sur le display local avant de démarrer X11 ou un lanceur. `ask` : proposer de démarrer le serveur X installé (VcXsrv ou Xming), `always` : le démarrer, `never` : seulement avertir. X11 démarre dans tous les cas. |
| `open_host` | `"localhost"` ou `"ip"` | `"localhost"` | Nom utilisé pour ouvrir un tunnel qui écoute sur une adresse de bouclage ou joker : `localhost`, ou `127.0.0.1` (`[::1]` en IPv6). |
| `open_default` | `"suggested"` ou `"browser"` | `"suggested"` | Ce qu'ouvre un tunnel sans `open.with`. `suggested` : l'application suggérée pour son port cible, sinon le navigateur. `browser` : toujours le navigateur. Un `open.with` enregistré et une demande explicite (`--with`, « Ouvrir avec… ») passent avant. |
| `standby` | booléen | `false` | Met en veille les tunnels locaux et dynamiques qui ne disent pas le contraire. |
| `standby_idle_s` | entier | `300` | Secondes sans connexion avant qu'un tunnel retourne en veille. Supérieur à 0. |
| `standby_notify` | `"none"`, `"visual"`, `"sound"` ou `"both"` | `"visual"` | Comment un réveil de veille, ou son échec, est signalé. |
| `standby_sound` | `"system"`, `"chime"`, `"marimba"` ou `"pluck"` | `"pluck"` | Le son joué dans les modes `sound` et `both`. |
| `standby_volume` | entier | `50` | Le volume des sons `chime`, `marimba` et `pluck`, en pourcentage, de 5 à 100. Le son `system` l'ignore et suit le volume du système. |
| `language` | `"system"`, `"en"` ou `"fr"` | `"system"` | Langue de la fenêtre, de l'icône et des notifications. `system` suit le système d'exploitation. |
| `check_for_updates` | booléen | `true` | Cherche une version plus récente à l'ouverture de la fenêtre. |

## `[[tunnels]]`

| Clé | Type | Défaut | Signification |
|---|---|---|---|
| `id` | chaîne ULID | générée | Identifiant stable. Deux tunnels ne peuvent pas en partager un. Quand il manque, Tunneler l'ajoute au fichier. |
| `host` | chaîne | obligatoire | Alias du `ssh_config`. Non vide. |
| `kind` | `"local"`, `"remote"` ou `"dynamic"` | obligatoire | Le sens de la redirection, comme `-L`, `-R` et `-D`. |
| `listen_port` | entier | obligatoire | Port d'écoute, de 1 à 65535. Local : sur cet ordinateur. Distant : sur le serveur. |
| `listen_address` | chaîne | `default_bind_address`, ou `default_remote_bind_address` pour un tunnel distant | Adresse d'écoute. Un nom d'hôte ne peut pas être ouvert avec « Ouvrir ». |
| `target_host` | chaîne | `"127.0.0.1"` | Où va le trafic. Local : vu depuis le serveur. Distant : vu depuis cet ordinateur. Interdit sur un tunnel dynamique. |
| `target_port` | entier | `listen_port` | Port où va le trafic, de 1 à 65535. Interdit sur un tunnel dynamique. |
| `tags` | tableau de chaînes | `[]` | Étiquettes. Ni espace, ni virgule, ni deux-points ; pas uniquement des chiffres ; pas de doublon dans un tunnel. |
| `standby` | booléen | `settings.standby` | `true` ou `false` pour ce tunnel. Ignoré sur un tunnel distant. |
| `open` | table | `{}` | Comment le tunnel s'ouvre. |

Deux tunnels locaux ou dynamiques ne peuvent pas écouter sur la même adresse et le même port. Deux tunnels distants du même hôte non plus.

### `open`

| Clé | Type | Défaut | Signification |
|---|---|---|---|
| `scheme` | `"http"` ou `"https"` | `"http"` | Schéma de l'URL ouverte dans le navigateur. |
| `path` | chaîne | `"/"` | Chemin de cette URL. Commence par `/`, pas par `//`, sans espace ni caractère de contrôle. |
| `host` | `"localhost"` ou `"ip"` | `settings.open_host` | Nom utilisé pour une adresse d'écoute de bouclage ou joker. Un tunnel qui écoute sur une IP précise ouvre cette IP. Un nom d'hôte n'est jamais utilisé. |
| `with` | chaîne | automatique | `"browser"` ou l'identifiant d'une application. Absent : l'application suggérée pour le port cible (sauf si `settings.open_default` vaut `"browser"`), sinon le navigateur. |

## `[[openers]]`

| Clé | Type | Défaut | Signification |
|---|---|---|---|
| `id` | chaîne | obligatoire | Minuscules, chiffres et tirets. Unique. `browser` est réservé. Le même identifiant qu'une application intégrée la remplace. |
| `name` | chaîne | obligatoire | Libellé affiché dans la fenêtre. |
| `command` | chaîne | obligatoire | Le programme, lancé sans shell. |
| `args` | tableau de chaînes | `[]` | Un argument par entrée. `{host}`, `{port}` et `{url}` sont remplacés ; tout autre `{…}` est refusé. |
| `ports` | tableau d'entiers | `[]` | Ports cibles pour lesquels cette application est suggérée. |

Voyez [Ouvrir un tunnel avec une application](../guide/applications).

## `[hosts.<alias>]`

| Clé | Type | Défaut | Signification |
|---|---|---|---|
| `x11_display` | chaîne | aucun | Display X local utilisé pour cet hôte, avant `settings.x11_display` et `$DISPLAY`. |

## `[[x11_launchers]]`

| Clé | Type | Signification |
|---|---|---|
| `host` | chaîne | L'alias sur lequel la commande s'exécute. |
| `name` | chaîne | Libellé affiché dans la fenêtre. Unique par hôte. |
| `command` | chaîne | La ligne de commande. |

Les trois sont obligatoires et non vides. Voyez [Transfert X11](../guide/x11).

## `state.toml`

Tunneler écrit `state.toml` lui-même : vous ne le modifiez pas. Il retient quels tunnels vous vouliez en marche, pour que `restore_on_launch` puisse les relancer, et quels fichiers de clé ont une passphrase dans le trousseau du système, pour que Tunneler puisse les oublier plus tard. Les tunnels temporaires n'y figurent jamais. Il reste à part de `tunnels.toml` pour que le fichier que vous modifiez à la main ne change pas à chaque démarrage de tunnel.
