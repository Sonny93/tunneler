export type GlobalShortcutKey = 'n' | 'i' | ',' | '?';

export type ShortcutDefinition = {
	keys: readonly string[];
	description: string;
};

export const GLOBAL_SHORTCUTS: readonly ShortcutDefinition[] = [
	{ keys: ['n'], description: 'Nouveau tunnel' },
	{ keys: ['i'], description: 'Importer des tunnels' },
	{ keys: [','], description: 'Ouvrir les paramètres' },
	{ keys: ['?'], description: 'Afficher les raccourcis clavier' },
	{ keys: ['Ctrl', 'K'], description: 'Ouvrir la palette de commandes' },
];

export const LIST_SHORTCUTS: readonly ShortcutDefinition[] = [
	{ keys: ['↑', '↓'], description: 'Tunnel précédent / suivant' },
	{ keys: ['Début', 'Fin'], description: 'Premier / dernier tunnel' },
	{ keys: ['Espace'], description: 'Démarrer ou arrêter le tunnel' },
	{ keys: ['Échap'], description: 'Fermer le panneau ou la fenêtre' },
];
