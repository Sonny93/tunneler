# Accessibilité

L'interface vise le niveau AA de WCAG 2.2. La ligne de commande suit les conventions `NO_COLOR` et hors terminal. Un changement de l'interface ou de la CLI n'est pas terminé tant que ces règles ne sont pas respectées.

## Définition de « terminé »

- Cela s'utilise au clavier seul.
- Chaque contrôle a un nom accessible correct.
- Aucune information ne passe par la couleur seule.
- Interface : `axe` ne signale aucune violation. CLI : la sortie est correcte avec `NO_COLOR=1` et sans terminal.

## Interface

### Clavier
- Toute action est atteignable sans souris, dans un ordre de tabulation logique.
- L'anneau de focus est toujours visible, avec un contraste d'au moins 3:1.
- Les listes se parcourent aux flèches (roving tabindex) : un seul arrêt de tabulation par liste.
- Les raccourcis sont listés dans l'aide (`?`) et ne sont jamais la seule voie : chaque action a aussi un bouton.

### Structure
- Utilisez d'abord le HTML natif : `button`, `form`, `label`, `table`, listes.
- Utilisez des landmarks (`nav`, `main`, `aside`) et des titres bien hiérarchisés.
- Les motifs ARIA complexes (modale, menu, combobox, switch, onglets, infobulle) viennent de `@minimalstuff/ui`. Si la bibliothèque est en défaut d'accessibilité, corrigez-la dans la bibliothèque plutôt que de la contourner dans l'application.

### Noms et états
- Un contrôle dit ce qu'il fait et dans quel état il est : l'interrupteur d'un tunnel se lit « Tunnel 3333 vers nova:3333, actif », pas « activé ».
- Un bouton réduit à une icône a toujours un libellé.
- Tout état ou toute santé est une icône plus du texte, jamais la couleur seule (« Reconnexion 2 », « Cible injoignable »).
- Le contraste du texte est d'au moins 4,5:1 et celui des composants d'au moins 3:1, dans les deux thèmes.

### Annonces
- Les changements d'état passent par une région `aria-live` polie, regroupés et limités en fréquence : démarrer un tag donne une annonce, pas vingt. Rien n'est annoncé au chargement.
- Le mode assertif est réservé aux erreurs bloquantes.
- Les invites (passphrase, clé d'hôte) prennent le focus.
- Le journal du panneau de détails reste silencieux (`aria-live="off"`).

### Modales
- Le focus est piégé à l'intérieur, Échap ferme, et le focus revient à l'élément qui l'a ouverte.
- Exception : l'alerte de clé d'hôte modifiée exige un choix explicite, donc Échap ne fait rien.

### Formulaires
- Des labels visibles.
- Les erreurs sont liées à leur champ (`aria-describedby`, `aria-invalid`), disent précisément ce qui ne va pas et ne dépendent pas du survol.

### Préférences du système
- `prefers-reduced-motion` : aucune animation n'est essentielle.
- `prefers-color-scheme` : les deux thèmes fonctionnent.
- `forced-colors` (contraste élevé de Windows) : badges, focus et icônes restent visibles.
- Zoom à 200 % ou grand texte : aucune perte de contenu et aucun défilement horizontal, jusqu'à la taille minimale de la fenêtre.

### Cibles
Les zones cliquables font au moins 24 × 24 px (WCAG 2.5.8).

## Zone de notification
- Les entrées du menu sont du texte et incluent l'état (« dev : 3/5 actifs »). Les tags sont cochables.
- L'infobulle résume l'état global, et chaque état a une forme d'icône distincte.
- Elle n'est jamais la seule voie : tout ce qu'elle fait existe dans l'interface et la CLI.

## Ligne de commande
- L'état est un symbole plus un mot (`✔ actif`, `✖ échec`), avec un repli ASCII quand le terminal n'est pas en UTF-8.
- Respectez `NO_COLOR`, `TERM=dumb` et `--color auto|always|never`. Couleurs, spinners et réécriture de ligne se coupent d'eux-mêmes hors terminal.
- Un spinner n'est jamais le seul signe de progression : affichez une ligne par étape.
- La sortie par défaut se lit ligne à ligne, sans tableaux en caractères de dessin que les lecteurs d'écran lisent mal. `--json` est stable pour les scripts.
- Les erreurs vont sur stderr avec la cause et une action suggérée. Les codes de sortie sont documentés.
- Les confirmations interactives peuvent être contournées (`--yes`), et rien n'attend une saisie hors terminal.
- Chaque sous-commande a un `--help` complet avec des exemples.

## Tests

Les tests automatiques couvrent `axe` sur chaque composant et écran, les rôles et noms accessibles, le focus des modales et des tiroirs, le contraste des jetons dans les deux thèmes, et la sortie de la CLI avec `NO_COLOR=1` et sans terminal. Ils ne voient ni un vrai moteur de rendu, ni un vrai lecteur d'écran, ni les préférences du système : ces points se vérifient à la main.

| | Linux | Windows |
|---|---|---|
| Webview | WebKitGTK | WebView2 |
| Lecteur d'écran | Orca | NVDA |

Testez les binaires de release, pas `pnpm tauri dev`. Les vérifications manuelles encore ouvertes sont listées dans la [feuille de route](/fr/roadmap/#a-tester).
