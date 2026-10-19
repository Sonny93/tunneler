# Audit accessibilité final (tâche 13.5)

Cible : WCAG 2.2 AA, checklist [PLAN §8](PLAN.md#8-accessibilité). Grille à remplir sur les binaires de release (zip Windows, tar.gz Linux), pas sur `pnpm tauri dev`.

Légende : ✔ conforme · ✖ écart (décrit dans « Écarts ») · ◐ partiel · — non applicable · ? à vérifier.

Déjà acquis par les tests automatiques (chaque composant et écran) : `axe` sans violation, rôles et noms accessibles, focus des modales/tiroirs, contraste des tokens dans les deux thèmes (`tokens.test.ts`). La grille ne couvre que ce que ces tests ne voient pas : vrai moteur de rendu, vrai lecteur d'écran, vraies préférences système.

## Environnements

| | Linux | Windows |
|---|---|---|
| Système | | |
| Moteur | WebKitGTK | WebView2 |
| Lecteur d'écran | Orca | NVDA |
| Version testée | | |
| Date | | |

## UI

| Critère (PLAN §8) | Linux | Windows | Comment vérifier |
|---|---|---|---|
| Clavier complet, ordre de tabulation logique | ✔ (6.14, avant phases 9-12) | ? | Tout faire sans souris : créer, éditer, démarrer, arrêter, supprimer, tags, réglages, import, palette, détails, config d'hôte |
| Focus toujours visible (anneau ≥ 3:1) | ? | ? | Thème clair et sombre, chaque contrôle |
| Roving tabindex (liste des tunnels, palette) | ✔ liste (6.14) · ? palette | ? | ↑/↓/Début/Fin ; palette : flèches + Entrée + Échap |
| Raccourcis documentés (`?`), jamais seule voie | ? | ? | `n`, `,`, `i`, `?`, Ctrl+K : chaque action a aussi un bouton |
| Landmarks et titres hiérarchisés | ✔ (6.14) | ? | Navigation par landmarks/titres du lecteur d'écran |
| Noms accessibles explicites (switch, boutons icône) | ✔ (6.14) | ? | Switch lu « Tunnel X, port vers hôte:port, état » |
| Jamais la couleur seule (état, santé, tray) | ✔ (6.14) | ? | Badges icône + texte |
| Contraste en mode contraste élevé Windows (`forced-colors`) | — | ? | Thème Contraste élevé : badges, focus, icônes visibles |
| Annonces `aria-live` throttlées, rien au chargement | ✔ (6.14) | ? | Démarrer un tag : une annonce groupée |
| Journal des détails silencieux (`aria-live="off"`) | ? | ? | Tiroir « Détails » ouvert sur un tunnel actif : le lecteur ne lit pas le journal tout seul |
| Invites (passphrase, clé d'hôte) prennent le focus | ✔ clé d'hôte (6.14) · ? passphrase | ? | Entrée valide l'invite de passphrase (non testé automatiquement) |
| Modales : focus piégé, Échap, retour du focus | ✔ (6.14) | ? | Échap inopérant sur « clé changée » |
| Formulaires : labels, erreurs liées et relues au focus | ◐ (erreur de champ non relue par WebKitGTK, 6.16) | ? | Soumettre le formulaire vide, tabuler sur les champs en erreur |
| Import : cases à cocher et avertissements lus | ? | ? | Script collé, avertissement « Aucun alias… », cases « Déjà configuré » |
| `prefers-reduced-motion` | ✔ (6.14) | ? | Réglage système « réduire les animations » |
| `prefers-color-scheme` | ? | ? | Thème système clair/sombre |
| Zoom 200 % / grand texte sans scroll horizontal | ? (6.13) | ? | Ctrl+= jusqu'à 200 %, fenêtre 1000×700 puis minimale 640×480 |
| Cibles ≥ 24×24 px | ? | ? | Inspecteur : boutons icône, cases, switchs |

## Tray

| Critère | Linux | Windows | Comment vérifier |
|---|---|---|---|
| Libellés textuels avec état (« dev — 3/5 actifs ») | ✔ (phase 7) | ? | Ouvrir le menu |
| Tags cochables, tooltip d'état global | ✔ (phase 7) | ? | Survol + lecteur d'écran sur la zone de notification |
| Forme d'icône distincte par état | ✔ (phase 7) | ? | Repos, actif, reconnexion, erreur |

## CLI

| Critère | Linux | Windows | Comment vérifier |
|---|---|---|---|
| Symboles + mots, repli ASCII hors UTF-8 | ? | ? | `tunneler status` ; `LANG=C tunneler status` |
| `NO_COLOR`, `TERM=dumb`, `--color` | ? | ? | `NO_COLOR=1 tunneler status`, `tunneler status \| cat` |
| Pas de spinner seul, messages d'étape | ? | ? | `tunneler up <tag>` |
| Sortie ligne à ligne, `--json` stable | ? | ? | `tunneler list`, `tunneler list --json` |
| Erreurs claires + codes de sortie documentés | ? | ? | `tunneler up inconnu` ; `tunneler --help` |
| Confirmations contournables, aucun prompt hors TTY | ? | ? | `tunneler quit < /dev/null` ; `tunneler quit --yes` |
| `--help` complet par sous-commande | ? | ? | `tunneler <cmd> --help` |

## Écarts

Écarts connus avant l'audit (6.14, 6.16) : bouton du menu d'actions exposé « combo box » par WebKitGTK ; erreur de champ `Input` non relue au focus (WebKitGTK) ; écarts de `@minimalstuff/ui` listés en 6.16, rien d'ouvert upstream.

| # | Plateforme | Critère | Constat | Suite |
|---|---|---|---|---|
| 1 | Toutes | Contraste des bordures de contrôles (1.4.11) | Cases, champs, boutons outline de `@minimalstuff/ui` sous 3:1 | Écart assumé : pas de surcharge, rendu préféré (2026-10-19) |
