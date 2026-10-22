# Ligne de commande

`tunneler` lance le daemon et le pilote. Le même programme fait les deux : il n'y a rien d'autre à installer.

```sh
tunneler                  # démarre le daemon, ou demande à celui qui tourne d'afficher sa fenêtre
tunneler up dev           # démarre tous les tunnels du tag « dev »
tunneler status           # ce qui tourne
tunneler down --all       # arrête tous les tunnels
tunneler quit             # arrête le daemon
```

`up` et `open` démarrent le daemon en arrière-plan quand il ne tourne pas. Les autres commandes qui parlent au daemon se terminent avec le code 3 quand il ne tourne pas. Chaque commande accepte `--help` pour son usage et ses exemples.

## Choisir des tunnels

Les tunnels n'ont pas de nom. `up`, `down`, `open` et `check` les sélectionnent de trois façons :

| Vous écrivez | Cela sélectionne |
|---|---|
| `3333` | Le tunnel qui écoute sur le port 3333. Quand plusieurs hôtes en ont un, écrivez `hôte:port`. |
| `nova:3333` | Le tunnel de l'hôte `nova` qui écoute sur le port 3333. |
| `dev` | Tous les tunnels portant le tag `dev`. La casse est ignorée. |

Un tag ne peut pas être composé uniquement de chiffres ni contenir `:` : les trois formes ne se confondent donc jamais. `--tag`, sur `up` et `down`, lit toutes les cibles comme des tags. `open` et `check` demandent une cible qui désigne un seul tunnel : un tag convient quand un seul tunnel le porte.

## Options globales

Elles valent pour toutes les commandes. Chacune a une variable d'environnement.

| Option | Variable | Signification |
|---|---|---|
| `--config <CHEMIN>` | `TUNNELER_CONFIG` | Le fichier des tunnels. |
| `--state <CHEMIN>` | `TUNNELER_STATE` | Le fichier qui retient quels tunnels tournent. |
| `--ssh-config <CHEMIN>` | `TUNNELER_SSH_CONFIG` | Lit ce `ssh_config` à la place de `~/.ssh/config` et de celui du système, comme `ssh -F`. |
| `--socket <CHEMIN>` | `TUNNELER_SOCKET` | Le socket ou le tube nommé sur lequel le daemon écoute. |
| `--no-tray` | `TUNNELER_NO_TRAY` | N'affiche pas l'icône dans la zone de notification. |
| `--color <QUAND>` | | `auto` (par défaut), `always` ou `never`. |
| `-V`, `--version` | | Affiche la version. |

## Commandes

### `tunneler`

Sans commande, démarre le daemon et son icône et ouvre la fenêtre, ou demande au daemon qui tourne déjà d'afficher sa fenêtre.

### `tunneler daemon`

Lance le daemon au premier plan. Les journaux vont dans des fichiers quotidiens, et sur stderr quand c'est un terminal. Vous en avez rarement besoin, puisque `tunneler up` démarre le daemon pour vous. Réglez `TUNNELER_LOG` sur un filtre comme `debug` pour plus de détails.

### `tunneler up <CIBLE>...`

Démarre des tunnels, et démarre d'abord le daemon si besoin. Il attend que chaque tunnel soit actif, et demande une passphrase, un mot de passe ou la confirmation d'une clé d'hôte quand il est lancé dans un terminal.

| Option | Signification |
|---|---|
| `--tag` | Lit les cibles uniquement comme des tags. |
| `--no-wait` | Rend la main dès que le démarrage est demandé. |
| `--timeout <SECONDES>` | Combien de temps attendre. 30 par défaut. |
| `-L <SPEC>` | Démarre une redirection locale temporaire. |
| `-R <SPEC>` | Démarre une redirection distante temporaire. |
| `-D <SPEC>` | Démarre un proxy SOCKS temporaire. |

```sh
tunneler up 3333              # le tunnel qui écoute sur 3333
tunneler up nova:3333         # idem, en nommant l'hôte
tunneler up dev               # tous les tunnels du tag dev
tunneler up --no-wait dev     # demande et rend la main aussitôt
tunneler up --timeout 60 dev  # attend jusqu'à une minute
```

Avec `-L`, `-R` ou `-D`, le seul argument est l'hôte du `ssh_config`, et la redirection utilise la syntaxe de `ssh` :

```sh
tunneler up nova -L 5000                      # local 5000 vers le port 5000 de nova
tunneler up nova -L 5000:db.internal:5432     # local 5000 vers db.internal:5432 via nova
tunneler up nova -R 9000:localhost:3000       # nova:9000 vers le port local 3000
tunneler up nova -D 1080                      # proxy SOCKS sur le port local 1080 via nova
```

`-L` et `-R` prennent `[adresse_d_écoute:]port[:hôte:port_hôte]`, `-D` prend `[adresse_d_écoute:]port`. Un port seul désigne le même port sur l'interface de bouclage de l'hôte. Un tunnel temporaire n'est jamais enregistré, et `tunneler down` ou la sortie du daemon le retire. Quand un tunnel enregistré fait déjà la même redirection, c'est lui qui est démarré.

### `tunneler down [CIBLE]...`

Arrête des tunnels. `--all` vise tous les tunnels, et `--tag` lit les cibles comme des tags.

```sh
tunneler down 3333
tunneler down dev
tunneler down --all
```

### `tunneler status`

Affiche la version du daemon et chaque tunnel avec son état, un par ligne.

```
Tunneler daemon 0.1.0: 0 of 3 tunnels active
↻ reconnecting (attempt 2)  3333 → nova:3333  [dev, web]
↻ reconnecting (attempt 2)  5432 → db.internal:5432 via nova  [dev]
○ stopped  1080 → SOCKS via nova
```

La sortie de la ligne de commande est en anglais. Avec `--json`, elle affiche une seule ligne de JSON, `{"daemonVersion": …, "tunnels": […], "settings": {…}, "tray": {…}}`, en camelCase, avec l'état, la santé de la cible et le trafic de chaque tunnel. Les heures de santé sont en UTC.

### `tunneler list`

Affiche les tunnels configurés. Avec `--json`, elle affiche `{"tunnels": […]}`.

### `tunneler open <CIBLE>`

Ouvre un tunnel local, et le démarre d'abord s'il est arrêté. Un tunnel s'ouvre avec son application enregistrée, sinon celle qui est suggérée pour son port cible, sinon le navigateur. `--with <APP>` prend `browser` ou l'identifiant d'une application à la place.

```sh
tunneler open 3333
tunneler open nova:3333
tunneler open 5999 --with browser
tunneler open 5999 --with tigervnc
```

Voyez [Ouvrir un tunnel avec une application](../guide/applications). Le code de sortie 8 signifie que le tunnel n'est pas devenu prêt ou que le programme n'a pas pu être lancé, et 9 que `--with` désigne une application indisponible : le message liste les identifiants disponibles.

### `tunneler check <CIBLE>`

Essaie une fois la cible d'un tunnel local ou distant en marche, et met à jour la santé de la cible que montre `status`. Elle ouvre une connexion et la referme aussitôt. Certains services comptent cela comme une connexion ratée : ne la lancez pas en boucle. Elle se termine avec 0 quand la cible répond et 7 quand elle ne répond pas.

```sh
tunneler check 3333
```

### `tunneler quit`

Arrête le daemon. Quand des tunnels sont actifs, elle demande d'abord confirmation. `-y`, `--yes` les arrête sans demander.

### `tunneler purge`

Supprime tout ce que Tunneler a laissé sur la machine, sauf les programmes. Voyez [Mises à jour et désinstallation](../guide/updates-and-uninstall#tunneler-purge).

| Option | Signification |
|---|---|
| `--dry-run` | Liste ce qui serait supprimé et ne supprime rien. |
| `-y`, `--yes` | Ne demande pas. Obligatoire hors terminal, sinon le code de sortie est 5. |
| `--keep-config` | Garde `tunnels.toml`, `state.toml` et les passphrases mémorisées. |

## Sortie

- `--json` existe sur `status` et `list`. La sortie tient sur une ligne, faite pour les scripts.
- La couleur est utilisée quand la sortie est un terminal qui la gère et que `NO_COLOR` n'est pas défini. `NO_COLOR` défini à une valeur non vide la désactive, et `--color never` aussi. `--color always` la force.
- Aucune information n'est portée par la seule couleur : les états sont du texte et un symbole.
- Les invites de passphrase, de mot de passe ou de clé d'hôte n'apparaissent que si la commande tourne dans un terminal.

## Codes de sortie

| Code | Signification |
|---|---|
| 0 | Succès. |
| 1 | Erreur. Le message sur stderr dit laquelle. |
| 2 | Usage invalide. |
| 3 | Le daemon ne tourne pas. |
| 4 | Des tunnels n'ont pas démarré ou sont bloqués par un autre programme (`up`). Un tunnel fourni par un `ssh` externe compte comme démarré. |
| 5 | Une confirmation est nécessaire mais impossible à demander : pas de terminal, passez `--yes` (`quit`, `purge`). |
| 6 | Délai dépassé. |
| 7 | La cible du tunnel ne répond pas (`check`). |
| 8 | Le tunnel n'est pas devenu prêt, ou le navigateur ou l'application n'a pas pu être lancé (`open`). |
| 9 | L'application donnée avec `--with` n'est pas disponible (`open`). |

## Variables d'environnement

| Variable | Signification |
|---|---|
| `NO_COLOR` | Désactive la couleur. |
| `TUNNELER_LOG` | Filtre de journalisation du daemon et des commandes, comme `debug`. |
| `TUNNELER_SOCKET`, `TUNNELER_CONFIG`, `TUNNELER_STATE`, `TUNNELER_SSH_CONFIG`, `TUNNELER_NO_TRAY` | Comme l'option correspondante. |
| `TUNNELER_UI_BINARY` | Le programme `tunneler-ui` que le daemon lance. |
