# Tunnels depuis devslot

devslot donne à chaque copie de travail d'un projet ses propres ports et son propre projet compose sur une machine. Quand vos instances tournent sur un hôte joint en SSH, Tunneler peut les lister et ouvrir les tunnels vers leurs ports en quelques clics, sans saisir chaque port.

## Découvrir les instances

1. Ouvrez le panneau d'import et choisissez l'onglet « Depuis devslot ».
2. Choisissez l'hôte, un alias de votre `ssh_config`, puis « Découvrir ».
3. Chaque instance apparaît avec ses ports. Les ports web sont cochés, sauf ceux inférieurs à 1024. Les ports de service (une base de données, par exemple) ne le sont pas. Un port qui a déjà un tunnel est désactivé, avec la raison.
4. Choisissez « Démarrer N tunnels ».

Les ports qu'une instance impose dans sa propre configuration peuvent être partagés avec d'autres instances : ils sont signalés comme imposés, ou comme ambigus quand plusieurs instances annoncent le même, pour que vous sachiez laquelle vous atteignez.

Tunneler exécute `ds ls --json` sur l'hôte, dans la session SSH qu'il partage déjà avec les tunnels de cet hôte. C'est la seule commande qu'il y lance, et seulement quand vous choisissez « Découvrir ». Une découverte qui dépasse 20 secondes est annulée, sans compter l'attente d'une passphrase ou d'un mot de passe.

Chaque tunnel découvert est un tunnel local qui écoute sur `127.0.0.1`, quelle que soit l'adresse d'écoute par défaut des réglages, et atteint le même port sur l'hôte. Si ce port est déjà utilisé sur votre ordinateur (par exemple par un devslot qui y tourne aussi, puisqu'il attribue les mêmes ports), ou qu'il est le port d'écoute d'un autre tunnel de Tunneler, même arrêté, Tunneler propose un port local libre (le port distant plus 10000, ou le suivant libre) et le remplit ; vous pouvez le changer dans la liste. Les liens que l'application construit (une URL absolue dans une page, par exemple) pointent toujours vers le port distant : ils cessent de marcher quand les deux ports diffèrent.

## Ouvrir une instance en une étape

Quand vous savez déjà quelle instance vous voulez, passez le panneau. Une commande, ou un lien, fait tout le trajet : elle retrouve l'instance sur l'hôte, ouvre les tunnels vers ses ports web et ouvre l'application dans votre navigateur.

```sh
tunneler devslot vega web-shop-098a
```

`vega` est l'alias de l'hôte dans votre `ssh_config`, tel que cet ordinateur le connaît. `web-shop-098a` est le nom de l'instance tel que `ds ls` l'affiche : une copie principale ou un worktree. Le lien a la forme `tunneler://devslot/vega/web-shop-098a`. Chaque segment est fait de lettres, de chiffres, de `.`, `_` et `-`, commence par une lettre ou un chiffre, et compte au plus 64 caractères (hôte) et 255 (instance). La commande et ses options sont dans la [référence de la ligne de commande](../reference/cli#tunneler-devslot-hote-instance).

Quand la configuration devslot du serveur contient `[tunneler] host = "<alias>"`, `ds info` affiche cette commande et ce lien à côté de l'URL de prévisualisation. Le serveur n'affiche que du texte : rien ne se passe sur votre ordinateur tant que vous n'avez pas lancé la commande ou cliqué sur le lien.

### Ce que ça fait

1. Exécute `ds ls --json` sur l'hôte, comme la découverte ci-dessus.
2. Retire les tunnels devslot périmés qui ne demandent aucune question : les temporaires, et les enregistrés avec `devslot_stale = "delete"`, sous la même règle de sécurité que plus haut. Les routes retirées sont listées. Un tunnel enregistré qui demanderait votre décision est laissé tel quel : la commande dit combien attendent, et c'est le panneau d'import qui sert à répondre.
3. Démarre un tunnel pour chaque port web de l'instance qui n'en a pas, comme dans le panneau d'import. Un port qui a déjà un tunnel devslot de cet hôte, qui écoute sur la boucle locale (`127.0.0.1`, `::1` ou `localhost`, ou l'adresse d'écoute par défaut des réglages quand le tunnel n'en indique pas), est réutilisé, et démarré s'il était arrêté. Tout autre tunnel qui transmet le port, un tunnel que vous avez configuré vous-même ou qui écoute sur `0.0.0.0`, est le vôtre : le port est ignoré et vous démarrez ce tunnel vous-même. Un tunnel écoute sur `127.0.0.1`, et prend un port local libre quand le port distant est utilisé sur cet ordinateur.
4. Ouvre l'application dans le navigateur, sur `http://<label>.localhost:<port>`, sauf avec `--no-open`.

Seuls les ports web sont tunnelés. Les services comme une base de données ne le sont pas, ni les ports inférieurs à 1024 (laissés au panneau d'import). Un port inférieur à 1024 est signalé comme ignoré, volontairement. Un port ignoré parce qu'il est utilisé et qu'aucun port libre n'a été trouvé, parce qu'un tunnel périmé le retient et attend votre décision, ou parce qu'un tunnel que vous avez configuré le transmet déjà, est signalé aussi et fait sortir la commande avec le code 4, comme un port dont le tunnel a échoué.

La lancer deux fois ne crée rien de nouveau : la seconde exécution retrouve les tunnels de la première et ne fait que rouvrir le navigateur. Quand une autre instance annonce le même port que l'application, le navigateur peut atteindre cette autre instance, et la sortie le dit.

Les tunnels sont temporaires ou enregistrés selon `devslot_persist`, comme dans le panneau d'import (voir [Temporaires ou enregistrés](#temporaires-ou-enregistres)). Sans réglage, ils sont temporaires.

### Le lien

Cliquer sur un lien `tunneler://devslot/<hôte>/<instance>` ouvre Tunneler, qui affiche toujours une confirmation nommant l'hôte et l'instance avant toute connexion SSH. N'importe quelle page web peut pointer vers un lien de ce type : la confirmation empêche une page d'ouvrir des connexions depuis votre ordinateur. Un lien qui ne respecte pas le format, ou dont l'hôte n'est pas un alias de votre `ssh_config`, affiche une erreur et rien d'autre ne se passe. « Ouvrir » ne répond qu'une seconde après l'apparition de la confirmation ou le retour de la fenêtre au premier plan : un clic destiné à la page qui a envoyé le lien ne peut pas tomber dessus. Tant qu'une confirmation est affichée, un nouveau lien est ignoré : répondez d'abord à la première.

Un navigateur demande en général une confirmation avant de passer un lien à une autre application. Un terminal ne rend pas toujours les liens `tunneler://` cliquables : la commande est la voie qui marche toujours.

Le lien fonctionne quand le programme a déclaré le schéma `tunneler` à votre système :

- l'installeur Windows et le paquet Debian le font pour vous ;
- pour l'archive Linux (`tar.gz`), lancez `tunneler desktop-entry install`, qui ajoute le schéma à l'entrée de menu qu'elle écrit ;
- le zip portable Windows, l'AppImage et macOS ne le déclarent pas : utilisez la commande.

L'alias vient de la configuration du serveur. Un second ordinateur qui connaît le même serveur sous un autre alias obtient des liens qui échouent avec un alias inconnu : tapez la commande avec l'alias de cet ordinateur.

## Où se trouve `ds`

Une commande lancée par SSH sans terminal a souvent un `PATH` plus court que votre shell : `~/.cargo/bin` ou les shims de mise peuvent manquer, et `ds` est alors « introuvable ». Indiquez le chemin complet sous l'hôte, dans `tunnels.toml` :

```toml
[hosts.nova]
devslot_path = "~/.cargo/bin/ds"
```

Le chemin est utilisé tel quel, protégé pour le shell distant. Exception : un `~/` en tête est développé par le shell distant. Sans `devslot_path`, Tunneler lance `ds`. Voir la [configuration](../reference/configuration#hosts-alias).

## Temporaires ou enregistrés

Par défaut, les tunnels sont temporaires : ils ne sont pas écrits dans `tunnels.toml` et disparaissent quand vous les arrêtez ou quand le daemon quitte. Activez « Garder ces tunnels » pour les enregistrer.

L'interrupteur part du réglage `devslot_persist`, qu'un hôte peut surcharger avec son propre `devslot_persist`. Les deux sont décrits dans la [référence de configuration](../reference/configuration).

Un tunnel enregistré reçoit deux tags : `devslot`, et `ds-<projet>` pour le projet de l'instance. Chaque caractère du nom du projet autre qu'une lettre, un chiffre, `.`, `_` ou `-` devient `-`, et le tag est coupé à 64 octets. Une instance sans nom de projet, ou dont le nom ne laisse rien une fois nettoyé, n'a que `devslot`.

Un tunnel temporaire ne porte aucun tag. Il affiche son projet à côté du badge « Temporaire », et Tunneler le reconnaît comme tunnel devslot tant que le daemon tourne. Quand vous en enregistrez un (« Enregistrer le tunnel » dans son menu), il reçoit les deux mêmes tags.

## Ouvrir un port web

Un port web d'instance a un nom de la forme `<label>.localhost`. Le tunnel ouvre `http://<label>.localhost:<port>` dans le navigateur, qui résout `.localhost` vers votre propre machine : chaque instance garde ainsi ses propres cookies. Les applications et les autres programmes ne sont pas concernés : ils se connectent toujours à l'adresse de boucle habituelle. Voir `open.host_name` dans la [référence de configuration](../reference/configuration#open).

L'URL utilise le port local. Quand il diffère du port distant, les liens que l'application construit pointent vers le port distant.

## Garder les tunnels à jour

Chaque découverte compare les instances aux tunnels devslot de cet hôte : les tunnels enregistrés qui portent le tag `devslot` et les temporaires issus d'une découverte. Seuls comptent les tunnels locaux qui visent `127.0.0.1` de l'hôte. Un tunnel est périmé quand aucune instance n'annonce plus son port distant, ou quand ce port appartient désormais à un autre projet ou à un autre nom d'hôte.

- **Les tunnels temporaires** périmés sont retirés aussitôt, et le résultat est annoncé.
- **Les tunnels enregistrés** périmés sont listés avec « Supprimer » et « Garder ». « Garder » le masque pour cette découverte. Avec `devslot_stale = "delete"`, un tunnel enregistré dont le port a disparu est supprimé sans demander. Un tunnel enregistré dont le port a changé de projet fait toujours l'objet d'une question.
- **Règle de sécurité** : quand l'hôte ne liste aucune instance, ou quand tous les tunnels devslot enregistrés de l'hôte seraient supprimés, on demande pour chacun. Une liste vide n'efface donc jamais vos tunnels.
- **Les ports imposés** que plusieurs instances partagent ne sont jamais touchés.

Seuls les tunnels conformes à ce que la découverte a renvoyé sont retirés : un tunnel que vous avez modifié entre-temps est laissé tel quel.

## En cas d'échec

| Message | Cause |
|---|---|
| `ds` introuvable | Le shell n'a pas `ds` dans son `PATH` (code de sortie 127) : renseignez `devslot_path`. |
| La commande a échoué | `ds` s'est terminé en erreur. La fin de sa sortie d'erreur est affichée. |
| Sortie invalide | `ds` a répondu quelque chose que Tunneler ne sait pas lire, ou plus de 1 Mio. |
| Délai dépassé | Pas de réponse en 20 secondes. |

Les erreurs de connexion, de clé d'hôte et d'authentification sont les mêmes que pour n'importe quel tunnel de cet hôte.
