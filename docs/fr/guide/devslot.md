# Tunnels depuis devslot

devslot donne à chaque copie de travail d'un projet ses propres ports et son propre projet compose sur une machine. Quand vos instances tournent sur un hôte joint en SSH, Tunneler peut les lister et ouvrir les tunnels vers leurs ports en quelques clics, sans saisir chaque port.

## Découvrir les instances

1. Ouvrez le panneau d'import et choisissez l'onglet « Depuis devslot ».
2. Choisissez l'hôte, un alias de votre `ssh_config`, puis « Découvrir ».
3. Chaque instance apparaît avec ses ports. Les ports web sont cochés, sauf ceux inférieurs à 1024. Les ports de service (une base de données, par exemple) ne le sont pas. Un port qui a déjà un tunnel est désactivé, avec la raison.
4. Choisissez « Démarrer N tunnels ».

Les ports qu'une instance impose dans sa propre configuration peuvent être partagés avec d'autres instances : ils sont signalés comme imposés, ou comme ambigus quand plusieurs instances annoncent le même, pour que vous sachiez laquelle vous atteignez.

Tunneler exécute `ds ls --json` sur l'hôte, dans la session SSH qu'il partage déjà avec les tunnels de cet hôte. C'est la seule commande qu'il y lance, et seulement quand vous choisissez « Découvrir ». Une découverte qui dépasse 20 secondes est annulée, sans compter l'attente d'une passphrase ou d'un mot de passe.

Chaque tunnel découvert est un tunnel local qui écoute sur `127.0.0.1`, quelle que soit l'adresse d'écoute par défaut des réglages, et atteint le même port sur l'hôte. Si ce port est déjà utilisé sur votre ordinateur (par exemple par un devslot qui y tourne aussi, puisqu'il attribue les mêmes ports), Tunneler propose un port local libre (le port distant plus 10000, ou le suivant libre) et le remplit ; vous pouvez le changer dans la liste. Les liens que l'application construit (une URL absolue dans une page, par exemple) pointent toujours vers le port distant : ils cessent de marcher quand les deux ports diffèrent.

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
