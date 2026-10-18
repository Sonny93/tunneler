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
	{ keys: ['←', '→'], description: 'Case, interrupteur et menu de la ligne' },
	{
		keys: ['Espace'],
		description: 'Démarrer ou arrêter le tunnel, cocher ou décocher la case',
	},
	{ keys: ['Maj', 'clic'], description: 'Sélectionner une plage de tunnels' },
	{
		keys: ['Ctrl', 'A'],
		description: 'Sélectionner tous les tunnels affichés',
	},
	{ keys: ['Échap'], description: 'Effacer la sélection' },
	{ keys: ['Échap'], description: 'Fermer le panneau ou la fenêtre' },
];
