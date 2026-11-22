# Déplacer ses tunnels vers un autre ordinateur

Vous avez configuré vos tunnels sur un ordinateur et vous les voulez sur un autre. Exportez-les dans un fichier, copiez le fichier, puis importez-le là-bas. L'import ne fait qu'ajouter des tunnels : il ne modifie ni ne supprime jamais un tunnel que vous avez déjà.

## Exporter

Dans la fenêtre, ouvrez le tiroir « Importer et exporter », puis l'onglet « Fichier ». La section « Exporter vers un fichier » a deux filtres facultatifs, « Uniquement ces hôtes » et « Uniquement ces tags ». Un tunnel est exporté dès qu'il correspond à l'un des filtres, et sans filtre tous les tunnels enregistrés le sont. « Exporter… » ouvre la boîte de dialogue du système qui demande où enregistrer le fichier. L'annuler ne fait rien.

Depuis un terminal :

```sh
tunneler export -o tunnels-export.toml
tunneler export --tag dev --host nova > dev-et-nova.toml
```

Sans `-o`, le fichier va sur la sortie standard. `-o` refuse un fichier existant, sauf avec `--force`. Voir [Ligne de commande](../reference/cli#tunneler-export).

### Ce que contient le fichier

Pour chaque tunnel enregistré : son hôte, son type, ses ports et ses adresses, ses tags, ses [options d'ouverture](./applications) et son choix de veille. Les tunnels suivent l'ordre de votre configuration. Le format est décrit dans [Configuration](../reference/configuration#fichier-d-export).

### Ce qu'il ne contient pas

- Les identifiants de tunnel : l'autre ordinateur donne de nouveaux identifiants aux siens.
- Les réglages, les applications que vous avez ajoutées, les lanceurs X11 et les réglages par hôte.
- Les mots de passe et les passphrases. Ils ne sont jamais dans votre configuration, donc rien ne peut fuiter, et on vous les redemande sur l'autre ordinateur.
- Les tunnels temporaires.

Les hôtes sont des alias de votre `ssh_config`. Le fichier ne transporte pas cette configuration : l'autre ordinateur doit avoir les mêmes alias, ou vous les remplacez à l'import (voir [Les hôtes que cet ordinateur ne connaît pas](#les-hotes-que-cet-ordinateur-ne-connait-pas)).

## Importer

Dans l'onglet « Fichier », « Choisir un fichier… » ouvre la boîte de dialogue du système et affiche un aperçu du fichier. Rien n'est ajouté tant que vous n'avez pas confirmé avec « Importer N tunnels ». Depuis un terminal :

```sh
tunneler import tunnels-export.toml
```

`tunneler import` affiche l'aperçu et demande confirmation, ce qui vous laisse le lire d'abord. Voir [Ligne de commande](../reference/cli#tunneler-import-fichier) pour les options, dont `--dry-run` pour seulement regarder.

Le fichier peut être produit par `tunneler export`, ou être un `tunnels.toml` copié tel quel. Dans le second cas, seuls les `[[tunnels]]` sont lus : les réglages, les applications, les lanceurs X11 et les réglages par hôte sont ignorés, et les identifiants sont remplacés par de nouveaux.

Les tunnels importés démarrent à l'arrêt.

### Ce que dit l'aperçu

Chaque tunnel du fichier est affiché avec sa route, ses tags et ce qui lui arrivera :

| État | Sens |
|---|---|
| Sera ajouté | Rien dans votre configuration ne s'y oppose. |
| Déjà configuré | Vous avez déjà un tunnel avec le même hôte, le même type, les mêmes adresses, les mêmes ports et la même cible. Il est ignoré. Quand le fichier lui donne des tags que le vôtre n'a pas, ils sont listés comme non appliqués : un import ne modifie jamais un tunnel. |
| Port déjà utilisé | Un autre de vos tunnels écoute déjà sur cette adresse et ce port (ou, pour un tunnel distant, sur ce port de cet hôte). Il est ignoré, et l'aperçu nomme le tunnel. Tunneler ne lui choisit pas un autre port. |
| Même route qu'un tunnel précédent du fichier | Le fichier le liste deux fois. Seul le premier peut être ajouté. |
| Invalide | L'entrée enfreint une règle de la configuration, par exemple un nom d'hôte refusé (voir [Noms d'hôte](../reference/configuration#noms-d-hote)). L'aperçu donne la raison. |

Si votre configuration change entre l'aperçu et la confirmation, Tunneler vérifie à nouveau avant d'écrire quoi que ce soit. Un tunnel qui n'est plus importable est ignoré et signalé.

### Les hôtes que cet ordinateur ne connaît pas

Quand aucune entrée `Host` de votre `ssh_config` ne nomme exactement un hôte du fichier, l'aperçu le signale. Un bloc avec joker comme `Host *.example` peut quand même correspondre : c'est un avertissement, pas une erreur, car Tunneler n'évalue pas votre `ssh_config` pour le savoir.

Pour utiliser à la place un autre alias de cet ordinateur, choisissez-le dans le choix « Utiliser cet alias pour nova » affiché pour l'hôte `nova` (ou « Garder nova » pour le laisser). L'aperçu est recalculé avec ce choix. Depuis un terminal, `--host-map ANCIEN=NOUVEAU` fait la même chose.

### Les applications absentes de cet ordinateur

Quand un tunnel s'ouvre avec une application que cet ordinateur n'a pas (ni le navigateur, ni une application que vous avez ajoutée, ni une application détectée), l'import abandonne ce choix et le dit. Le tunnel est ajouté et s'ouvre avec l'application habituelle pour son port.

### Les tunnels décochés au départ

Certains tunnels sont risqués à ajouter sans les regarder. Ils peuvent être importés, mais ils ne sont pas cochés par défaut, et l'aperçu en donne la raison en texte :

- **Il écoute au-delà de cet ordinateur.** Un tunnel local ou dynamique dont l'adresse d'écoute n'est pas une adresse de bouclage peut être atteint par les autres machines de votre réseau.
- **Il envoie des connexions distantes ailleurs.** Un tunnel distant dont la cible n'est pas cet ordinateur laisse le serveur joindre une autre machine depuis la vôtre.
- **Il écoute au-delà du bouclage du serveur.** Un tunnel distant qui écoute sur une adresse du serveur autre que le bouclage, comme `0.0.0.0`, peut être atteint par d'autres machines là-bas quand le `GatewayPorts` du serveur le permet.
- **Ses tags démarrent au lancement.** Un tag du tunnel figure dans « Tags démarrés au lancement » ici : le tunnel démarrerait donc seul au prochain lancement de Tunneler.

Cochez-les vous-même dans la fenêtre. En ligne de commande, ils sont marqués `not selected` et laissés de côté : passez `--include-risky` pour les importer aussi (le plan les marque `will import (risky)` et la question dit combien sont risqués), ou modifiez le fichier pour retirer ce qui les rend risqués.

## Un fichier venu de quelqu'un d'autre

Traitez un fichier que vous n'avez pas fait vous-même comme un script : lisez l'aperçu. Un tunnel importé peut ouvrir un port sur votre ordinateur, ou faire se connecter un serveur à une machine choisie par l'auteur du fichier. C'est pourquoi les tunnels risqués démarrent décochés, et pourquoi l'import ne touche jamais à ce que vous avez. Tunneler refuse aussi tout le fichier s'il contient des caractères de contrôle ou des marques de direction invisibles dans une valeur, qui pourraient faire lire l'aperçu autrement que ce qu'il fait, et il refuse les noms d'hôte qui contiennent autre chose que des lettres, des chiffres et `. _ - : @ [ ]` (voir [Noms d'hôte](../reference/configuration#noms-d-hote)).

## Limites

Un fichier fait au plus 256 Kio et contient au plus 1000 tunnels. Un fichier plus gros est refusé avant d'être lu. L'export refuse d'écrire plus de 256 Kio, dans la fenêtre comme en ligne de commande : filtrez par hôte ou par tag pour exporter une partie de vos tunnels, puis les autres dans un autre fichier. L'aperçu peut aussi refuser un fichier dont le plan est trop gros pour être affiché, par exemple un très gros fichier avec des noms d'hôte très longs.
