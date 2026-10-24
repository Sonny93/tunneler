# Mises à jour et désinstallation

## Mises à jour

Quand la fenêtre s'ouvre, Tunneler demande une fois à GitHub s'il existe une version plus récente. Si la vérification échoue, par exemple hors ligne, rien n'est affiché. Rien d'autre dans Tunneler ne contacte le réseau de lui-même, et rien n'est installé sans que vous cliquiez.

Quand une version plus récente existe, un bandeau montre ses notes de version, un lien « Voir la version » et un bouton « Masquer » qui le cache jusqu'à la prochaine ouverture de la fenêtre.

| Forme | Ce que propose le bandeau |
|---|---|
| Installeur Windows | « Installer et redémarrer ». Il demande confirmation quand des tunnels tournent : ils s'arrêtent, puis redémarrent ensuite si « Rétablir les tunnels actifs au lancement » est activé. Il télécharge l'installeur avec un texte de progression, vérifie sa signature, arrête le daemon, lance l'installeur et relance Tunneler. |
| Zip Windows, archive Linux, paquet Debian, AppImage | Le lien seulement. Téléchargez la nouvelle version et remplacez l'ancienne vous-même. Pour le paquet : `sudo apt install ./tunneler_<version>_amd64.deb`. |

L'AppImage ne se met jamais à jour toute seule, car elle ne contient pas le daemon.

Pour vérifier à tout moment, même avec la vérification automatique désactivée, cliquez sur « Vérifier maintenant » sous « Mises à jour » dans l'onglet Général des paramètres, ou lancez « Vérifier les mises à jour » depuis la palette de commandes (`Ctrl+K`). Une ligne à côté du bouton dit si vous avez la dernière version, quelle version est disponible, ou que la vérification a échoué. Une version trouvée ainsi réaffiche le bandeau, même si vous l'aviez masqué.

Désactivez la vérification automatique avec « Vérifier les mises à jour à l'ouverture » dans les paramètres, ou dans `tunnels.toml` :

```toml
[settings]
check_for_updates = false
```

Les mises à jour sont signées. Un Tunneler installé n'accepte que les mises à jour signées avec la clé fournie avec lui.

Une installation Windows pour tous les utilisateurs demande les droits administrateur à chaque mise à jour. L'installeur arrête un daemon en cours avant de copier les fichiers.

## Où vont les fichiers

Tunneler écrit `tunnels.toml`, `state.toml`, des journaux et les données web de la fenêtre. [Configuration](../reference/configuration#emplacement-des-fichiers) donne les chemins pour chaque système. Les passphrases que vous avez demandé de mémoriser vont dans le trousseau du système, et « Lancer Tunneler à l'ouverture de session » ajoute une entrée que le système conserve.

## Mode portable

Le zip Windows et l'archive Linux contiennent un fichier vide nommé `portable` à côté des programmes. Tant qu'il est là, Tunneler garde ses fichiers dans ce dossier au lieu de vos dossiers d'utilisateur :

| Fichier | Portable | Installé sous Linux |
|---|---|---|
| `tunnels.toml` | à côté des programmes | `~/.config/tunneler/` |
| `state.toml` | `data/` | `~/.local/state/tunneler/` |
| Journaux | `data/logs/` | `~/.local/share/tunneler/logs/` |
| Données web de la fenêtre | `data/webview/` | `~/.local/share/com.minimalstuff.tunneler/` |

Supprimez le fichier `portable` pour revenir aux dossiers d'utilisateur. Les fichiers déjà écrits ne sont pas déplacés. `--config` et `--state`, ou `TUNNELER_CONFIG` et `TUNNELER_STATE`, l'emportent sur les deux.

L'entrée de lancement à l'ouverture de session et les passphrases du trousseau du système restent hors du dossier. `tunneler purge` les supprime.

Gardez `tunneler` et `tunneler-ui` dans le même dossier : le daemon cherche `tunneler-ui` à côté de lui, puis dans le `PATH`, puis au chemin donné par `TUNNELER_UI_BINARY`.

## Désinstaller

### `tunneler purge`

Supprimer les programmes laisse vos fichiers derrière. `tunneler purge` supprime ce que Tunneler a laissé sur la machine, sauf les programmes. Il arrête le daemon, puis supprime :

- l'entrée de lancement à l'ouverture de session ;
- les journaux et les données web de la fenêtre ;
- `tunnels.toml` et `state.toml` ;
- les passphrases du trousseau du système que `state.toml` liste ;
- les dossiers `tunneler` restés vides.

Il ne touche jamais aux programmes, au fichier `portable`, au dossier d'installation ni à `~/.ssh`, et ne supprime jamais un fichier donné avec `--config` ou `--state`.

```sh
tunneler purge --dry-run      # liste ce qui serait supprimé, ne supprime rien
tunneler purge                # liste, demande confirmation, supprime
tunneler purge --yes          # sans question ; obligatoire hors terminal, sinon code de sortie 5
tunneler purge --keep-config  # garde tunnels.toml, state.toml et les passphrases
```

Utilisez `--keep-config` quand vous comptez réinstaller et retrouver vos tunnels.

### Installeur Windows

Ouvrez « Paramètres », « Applications », « Tunneler », puis « Désinstaller ». La page de confirmation a une case à cocher : « Supprimer aussi mes tunnels, réglages et passphrases mémorisées ».

- Cochée : l'installeur lance `tunneler purge --yes`, et tout disparaît.
- Décochée : il lance `tunneler purge --yes --keep-config`. Vos tunnels, vos réglages et vos passphrases restent pour une réinstallation, et le reste est nettoyé.

Une mise à jour ne purge jamais. Une installation pour tous les utilisateurs se désinstalle avec les droits administrateur, et son désinstalleur ne nettoie que les données de l'utilisateur qui le lance, puisque ces données sont propres à chacun.

### Debian et Ubuntu

Lancez d'abord `tunneler purge`, puis supprimez le paquet. `apt` ne touche jamais au dossier personnel : le paquet ne peut pas nettoyer vos fichiers lui-même.

```sh
tunneler purge
sudo apt remove tunneler
```

### Portable

Lancez `tunneler purge`, puis supprimez le dossier.
