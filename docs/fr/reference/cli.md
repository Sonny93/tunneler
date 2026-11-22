# Ligne de commande

`tunneler` lance le daemon et le pilote. Le même programme fait les deux : il n'y a rien d'autre à installer.

```sh
tunneler                  # démarre le daemon, ou demande à celui qui tourne d'afficher sa fenêtre
tunneler up dev           # démarre tous les tunnels du tag « dev »
tunneler status           # ce qui tourne
tunneler down --all       # arrête tous les tunnels
tunneler quit             # arrête le daemon
```

`up`, `open` et `import` démarrent le daemon en arrière-plan quand il ne tourne pas. Les autres commandes qui parlent au daemon se terminent avec le code 3 quand il ne tourne pas. Chaque commande accepte `--help` pour son usage et ses exemples.

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

### `tunneler devslot <HÔTE> <INSTANCE>`

Ouvre les tunnels vers les ports web d'une instance [devslot](../guide/devslot) et l'ouvre dans le navigateur. `HÔTE` est un alias de votre `ssh_config` et `INSTANCE` le nom de l'instance tel que `ds ls` l'affiche. Le daemon est démarré quand il ne tourne pas. Si un mot de passe, une passphrase ou une clé d'hôte inconnue doit être confirmé, la question est posée dans le terminal quand il y en a un. La relancer ne crée rien de nouveau.

Les deux arguments comptent de 1 à 64 caractères (`HÔTE`) ou de 1 à 255 (`INSTANCE`), faits de lettres, de chiffres, de `.`, `_` et `-`, le premier étant une lettre ou un chiffre. Toute autre valeur est refusée avec le code de sortie 2, avant toute connexion.

| Option | Signification |
|---|---|
| `--no-open` | Démarre les tunnels et laisse le navigateur tranquille. |
| `--json` | Affiche le résultat en une ligne de JSON. |

Seuls les ports web sont tunnelés, et pas ceux inférieurs à 1024 (ignorés volontairement, ce n'est pas un échec). La sortie texte donne une ligne par port (démarré, déjà en place, ignoré avec la raison, en échec avec le message), les routes périmées retirées d'abord, puis l'adresse ouverte. Avec `--json`, la ligne est un objet avec les clés `host`, `instance`, `started`, `existing`, `skipped`, `failed`, `removed`, `needsReview`, `appPortShared` et, quand elles s'appliquent, `appUrl`, `opened` et `openError`. `started` et `existing` listent des `{ remotePort, listenPort, tunnelId }`, `skipped` des `{ remotePort, reason }` avec une raison parmi `localPortBusy`, `needsReview`, `existingTunnel` (un tunnel que vous avez configuré, ou qui écoute au-delà de la boucle locale, transmet déjà le port : démarrez-le vous-même) ou `privilegedPort`, `failed` des `{ remotePort, message }`, et `removed` des `{ listenPort, remotePort, isTemporary }`.

Codes de sortie : 0 quand l'instance est ouverte, 1 pour une erreur (hôte inconnu, instance absente de la liste, échec de connexion ou de `ds`), 4 quand des ports web ont échoué ou ont été ignorés parce que le port local est occupé, qu'un tunnel périmé attend votre décision ou qu'un tunnel que vous avez configuré transmet déjà le port, 6 quand la commande sur l'hôte a dépassé le délai, 8 quand le navigateur n'a pas pu être ouvert.

```sh
tunneler devslot vega web-shop-098a
tunneler devslot vega web-shop-098a --no-open
tunneler devslot vega web-shop-098a --json
```

### `tunneler export`

Écrit vos tunnels enregistrés dans un fichier que vous pouvez [importer sur un autre ordinateur](../guide/export-import). Le daemon doit tourner : sinon la commande se termine avec le code 3.

| Option | Signification |
|---|---|
| `--host <HÔTE>` | Seulement les tunnels de cet hôte. À répéter pour plusieurs hôtes. |
| `--tag <TAG>` | Seulement les tunnels portant ce tag. À répéter pour plusieurs tags. |
| `-o`, `--output <FICHIER>` | Écrit dans ce fichier au lieu de la sortie standard. Un fichier ou un lien symbolique existant est refusé. Sous Unix, le nouveau fichier n'est lisible que par vous (mode 0600). |
| `--force` | Remplace le fichier donné avec `-o` s'il existe. Demande `-o`. |
| `--json` | Affiche le résultat en une ligne de JSON. |

Un tunnel est exporté dès qu'il correspond à l'un des filtres `--host` et `--tag`. Sans filtre, tous les tunnels enregistrés le sont. Les tunnels temporaires ne sont jamais exportés. Un export de plus de 256 Kio (la limite de l'import) est refusé avec le message `the export is larger than 256 KiB; filter by host or tag` et l'indication `pass --host or --tag to export a part of the tunnels, then the others in another file`. Le format est décrit dans [Configuration](./configuration#fichier-d-export).

```sh
tunneler export > tous-les-tunnels.toml
tunneler export --tag dev --tag web -o dev-web.toml
tunneler export --host nova --host vega -o deux-hotes.toml --force
```

Avec `--json`, la sortie est `{"contents": "…", "tunnelCount": 3}`, où `contents` est le fichier sous forme de chaîne. Avec `-o` et `--json`, le fichier est écrit et la sortie est `{"path": "…", "tunnelCount": 3}`.

### `tunneler import <FICHIER>`

Ajoute à votre configuration les tunnels d'un fichier produit par `tunneler export`, ou d'un `tunnels.toml`. `FICHIER` est un chemin, ou `-` pour l'entrée standard. Le daemon est démarré quand il ne tourne pas. La commande ne modifie ni ne supprime jamais un tunnel que vous avez déjà : voyez [Déplacer ses tunnels vers un autre ordinateur](../guide/export-import) pour ce qu'elle ajoute et ce qu'elle ignore.

Elle affiche d'abord le plan : pour chaque tunnel du fichier, sa route, ses tags, son état (sera ajouté, déjà configuré avec les tags non appliqués, port déjà utilisé, répété dans le fichier, invalide), ses risques et ses avertissements, et s'il est sélectionné. Ensuite :

| Situation | Ce qui se passe |
|---|---|
| `--dry-run` | Le plan est affiché et rien n'est ajouté. Code de sortie 0. |
| `--yes` | Les tunnels sélectionnés par défaut sont ajoutés sans demander. |
| Refus à la question | Elle affiche `Nothing was imported.` et se termine avec le code 0. |
| Dans un terminal | Elle demande `Import N tunnels? [y/N]` et ajoute les tunnels sélectionnés par défaut. La réponse est lue sur le terminal, donc `tunneler import -` fonctionne. |
| Pas de terminal et pas de `--yes` | Le plan est affiché et rien n'est ajouté. Code de sortie 5. |

Les tunnels risqués (écoute au-delà de cet ordinateur, tunnel distant vers une cible qui n'est pas cet ordinateur ou qui écoute au-delà du bouclage du serveur, tag qui démarre au lancement) ne sont pas sélectionnés par défaut, et aucune option ne les sélectionne : modifiez le fichier, ou passez par la fenêtre. Les tunnels ajoutés démarrent à l'arrêt.

| Option | Signification |
|---|---|
| `--host-map <ANCIEN=NOUVEAU>` | Utilise l'alias `NOUVEAU` de cet ordinateur pour l'hôte `ANCIEN` du fichier. À répéter pour plusieurs hôtes. |
| `--dry-run` | Affiche le plan et n'ajoute rien. |
| `-y`, `--yes` | Ne demande pas. |
| `--json` | Affiche le résultat en une ligne de JSON. Elle ne demande jamais : sans `--yes`, elle affiche le plan et se termine avec le code 5. |

Le fichier est lu jusqu'à 256 Kio et 1000 tunnels. Une entrée plus grosse est refusée sans être lue en entier.

```sh
tunneler import dev-web.toml --dry-run
tunneler import dev-web.toml
tunneler import dev-web.toml --host-map nova=nova-maison --yes
ssh vega cat .config/tunneler/tunnels.toml | tunneler import - --dry-run
```

Avec `--json`, la sortie est le plan avec `--dry-run` ou sans `--yes` (dans ce second cas, avec le code 5), et `{"plan": …, "report": …}` avec `--yes`. Le plan est `{"source", "entries", "unknownHosts"}` : `source` vaut `export` ou `store`, `unknownHosts` liste les hôtes du fichier qu'aucune entrée `Host` de votre `ssh_config` ne nomme exactement et qui n'ont pas de correspondance, et chaque entrée a :

- `index` : sa position dans le fichier, à partir de 0.
- `tunnel` : ce qui serait créé, après la correspondance des hôtes.
- `fileHost` : l'hôte tel que le fichier l'écrit.
- `effectiveListenAddress` : l'adresse sur laquelle il écouterait.
- `status` : `{"kind": "importable"}`, `{"kind": "alreadyConfigured", "existingId", "ignoredTags"}`, `{"kind": "listenConflict", "holderId"}`, `{"kind": "duplicateInFile", "earlierIndex"}` ou `{"kind": "invalid", "messages"}`.
- `risks` : une liste de `{"kind": "nonLoopbackListen", "address"}` (aussi pour un tunnel distant qui écoute sur une adresse du serveur autre que le bouclage), `{"kind": "remoteNonLoopbackTarget", "targetHost"}` ou `{"kind": "startsOnLaunch", "tags"}`.
- `warnings` : une liste de `{"kind": "unknownHost", "host"}` ou `{"kind": "openerDropped", "opener"}`.
- `selectedByDefault` : `true` quand l'état est `importable` et qu'il n'y a aucun risque.

Le rapport est `{"added": [{"index", "id"}], "skipped": [{"index", "status"}]}`, où `status` est l'un des états ci-dessus, tel qu'il était au moment de l'ajout.

Codes de sortie de `export` et `import` :

| Code | Quand |
|---|---|
| 0 | Exporté, importé, rien à importer, refus à la question, ou `--dry-run`. |
| 1 | L'export dépasse 256 Kio (`export`). Le fichier est illisible, trop gros, n'est pas de l'UTF-8 valide, contient des caractères de contrôle, ou est refusé comme n'étant pas un fichier Tunneler ou venant d'une version plus récente. Aussi : un fichier de sortie existant sans `--force`, une erreur d'écriture ou une erreur du daemon. |
| 2 | Usage invalide, comme un `--host-map` qui n'est pas `ANCIEN=NOUVEAU` ou qui nomme deux fois le même `ANCIEN`. |
| 3 | Le daemon ne tourne pas (`export`). |
| 5 | Une confirmation est nécessaire mais il n'y a ni terminal ni `--yes`. |

### `tunneler desktop-entry <install|uninstall>`

Linux seulement. Ajoute Tunneler au menu des applications, ou l'en retire. L'archive n'a pas d'installeur : c'est ainsi qu'elle obtient une entrée de menu. Voyez [Premiers pas](../guide/getting-started#linux).

`install` écrit `com.minimalstuff.tunneler.desktop` et trois icônes sous le dossier de données de l'utilisateur (`$XDG_DATA_HOME`, en général `~/.local/share`), avec le `tunneler-ui` qui est à côté de `tunneler`. Relancez-la après une mise à jour qui déplace les programmes dans un autre dossier. Elle fait aussi pointer une entrée de lancement à l'ouverture de session existante vers ce `tunneler`, sans jamais en créer une. Elle refuse de s'exécuter depuis `/usr` (mais pas `/usr/local`), où un paquet fournit sa propre entrée, et n'écrase jamais un fichier `.desktop` que Tunneler n'a pas écrit. Elle n'écrit que les fichiers absents ou différents, affiche `Wrote` ou `Unchanged` pour chacun, et ne lance rien. L'entrée déclare aussi le schéma de lien `tunneler://` (`MimeType=x-scheme-handler/tunneler;`, et `%u` sur la ligne `Exec`), voir [Ouvrir une instance en une étape](../guide/devslot#ouvrir-une-instance-en-une-etape). Après l'avoir écrite, `install` lance `update-desktop-database` sur le dossier des applications quand ce programme existe, et ignore un échec.

`uninstall` supprime l'entrée et les icônes. Elle refuse de supprimer un fichier `.desktop` que Tunneler n'a pas écrit.

Sur les autres systèmes, les deux échouent avec le code de sortie 2. Un refus (pas de `tunneler-ui` à côté de `tunneler`, un fichier en travers, un dossier non inscriptible) sort avec 1.

```sh
tunneler desktop-entry install
tunneler desktop-entry uninstall
```

## Sortie

- `--json` existe sur `status`, `list`, `devslot`, `export` et `import`. La sortie tient sur une ligne, faite pour les scripts.
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
| 4 | Des tunnels n'ont pas démarré ou sont bloqués par un autre programme (`up`), ou des ports web de l'instance ont échoué, ou ont été ignorés parce que le port local est occupé, qu'un tunnel périmé attend une décision ou qu'un tunnel que vous avez configuré transmet déjà le port (`devslot`). Un tunnel fourni par un `ssh` externe compte comme démarré. |
| 5 | Une confirmation est nécessaire mais impossible à demander : pas de terminal, passez `--yes` (`quit`, `purge`, `import`). |
| 6 | Délai dépassé, ou la commande sur l'hôte a dépassé le délai (`devslot`). |
| 7 | La cible du tunnel ne répond pas (`check`). |
| 8 | Le tunnel n'est pas devenu prêt, ou le navigateur ou l'application n'a pas pu être lancé (`open`, `devslot`). |
| 9 | L'application donnée avec `--with` n'est pas disponible (`open`). |

## Variables d'environnement

| Variable | Signification |
|---|---|
| `NO_COLOR` | Désactive la couleur. |
| `TUNNELER_LOG` | Filtre de journalisation du daemon et des commandes, comme `debug`. |
| `TUNNELER_SOCKET`, `TUNNELER_CONFIG`, `TUNNELER_STATE`, `TUNNELER_SSH_CONFIG`, `TUNNELER_NO_TRAY` | Comme l'option correspondante. |
| `TUNNELER_UI_BINARY` | Le programme `tunneler-ui` que le daemon lance. |
