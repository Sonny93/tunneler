import { defineConfig, type DefaultTheme } from 'vitepress'

const BASE = '/tunneler/'
const REPOSITORY_URL = 'https://github.com/Sonny93/tunneler'

function englishNav(): DefaultTheme.NavItem[] {
	return [
		{ text: 'Guide', link: '/guide/getting-started', activeMatch: '/guide/' },
		{ text: 'Reference', link: '/reference/configuration', activeMatch: '/reference/' },
		{ text: 'Roadmap', link: '/roadmap/', activeMatch: '/roadmap/' },
		{ text: 'Contributing', link: '/contributing/development', activeMatch: '/contributing/' },
	]
}

function englishSidebar(): DefaultTheme.Sidebar {
	return {
		'/guide/': [
			{
				text: 'Guide',
				items: [
					{ text: 'Getting started', link: '/guide/getting-started' },
					{ text: 'Tunnels', link: '/guide/tunnels' },
					{ text: 'Opening tunnels with an application', link: '/guide/applications' },
					{ text: 'X11 forwarding', link: '/guide/x11' },
					{ text: 'Tunnels started outside Tunneler', link: '/guide/external-tunnels' },
					{ text: 'Updates and uninstall', link: '/guide/updates-and-uninstall' },
				],
			},
		],
		'/reference/': [
			{
				text: 'Reference',
				items: [
					{ text: 'Configuration', link: '/reference/configuration' },
					{ text: 'Command line', link: '/reference/cli' },
				],
			},
		],
		'/roadmap/': [
			{
				text: 'Roadmap',
				items: [
					{ text: 'Roadmap', link: '/roadmap/' },
					{ text: 'Design decisions', link: '/roadmap/decisions' },
				],
			},
		],
		'/contributing/': [
			{
				text: 'Contributing',
				items: [
					{ text: 'Development', link: '/contributing/development' },
					{ text: 'Accessibility', link: '/contributing/accessibility' },
				],
			},
		],
	}
}

function frenchNav(): DefaultTheme.NavItem[] {
	return [
		{ text: 'Guide', link: '/fr/guide/getting-started', activeMatch: '/fr/guide/' },
		{ text: 'Référence', link: '/fr/reference/configuration', activeMatch: '/fr/reference/' },
		{ text: 'Feuille de route', link: '/fr/roadmap/', activeMatch: '/fr/roadmap/' },
		{
			text: 'Contribuer',
			link: '/fr/contributing/development',
			activeMatch: '/fr/contributing/',
		},
	]
}

function frenchSidebar(): DefaultTheme.Sidebar {
	return {
		'/fr/guide/': [
			{
				text: 'Guide',
				items: [
					{ text: 'Prise en main', link: '/fr/guide/getting-started' },
					{ text: 'Tunnels', link: '/fr/guide/tunnels' },
					{ text: 'Ouvrir un tunnel avec une application', link: '/fr/guide/applications' },
					{ text: 'Transfert X11', link: '/fr/guide/x11' },
					{ text: 'Tunnels lancés hors de Tunneler', link: '/fr/guide/external-tunnels' },
					{ text: 'Mises à jour et désinstallation', link: '/fr/guide/updates-and-uninstall' },
				],
			},
		],
		'/fr/reference/': [
			{
				text: 'Référence',
				items: [
					{ text: 'Configuration', link: '/fr/reference/configuration' },
					{ text: 'Ligne de commande', link: '/fr/reference/cli' },
				],
			},
		],
		'/fr/roadmap/': [
			{
				text: 'Feuille de route',
				items: [
					{ text: 'Feuille de route', link: '/fr/roadmap/' },
					{ text: 'Décisions de conception', link: '/fr/roadmap/decisions' },
				],
			},
		],
		'/fr/contributing/': [
			{
				text: 'Contribuer',
				items: [
					{ text: 'Développement', link: '/fr/contributing/development' },
					{ text: 'Accessibilité', link: '/fr/contributing/accessibility' },
				],
			},
		],
	}
}

const frenchSearchTranslations = {
	button: { buttonText: 'Rechercher', buttonAriaLabel: 'Rechercher' },
	modal: {
		displayDetails: 'Afficher le détail',
		resetButtonTitle: 'Réinitialiser la recherche',
		backButtonTitle: 'Fermer la recherche',
		noResultsText: 'Aucun résultat pour',
		footer: {
			selectText: 'sélectionner',
			selectKeyAriaLabel: 'entrée',
			navigateText: 'naviguer',
			navigateUpKeyAriaLabel: 'flèche du haut',
			navigateDownKeyAriaLabel: 'flèche du bas',
			closeText: 'fermer',
			closeKeyAriaLabel: 'échap',
		},
	},
}

export default defineConfig({
	title: 'Tunneler',
	base: BASE,
	cleanUrls: true,
	lastUpdated: false,
	head: [['link', { rel: 'icon', type: 'image/svg+xml', href: `${BASE}logo.svg` }]],
	themeConfig: {
		logo: '/logo.svg',
		socialLinks: [{ icon: 'github', link: REPOSITORY_URL }],
		search: {
			provider: 'local',
			options: { locales: { fr: { translations: frenchSearchTranslations } } },
		},
	},
	locales: {
		root: {
			label: 'English',
			lang: 'en',
			description:
				'SSH tunnel manager with an embedded SSH client, a tray daemon and an on-demand UI.',
			themeConfig: {
				nav: englishNav(),
				sidebar: englishSidebar(),
				editLink: {
					pattern: `${REPOSITORY_URL}/edit/main/docs/:path`,
					text: 'Edit this page on GitHub',
				},
			},
		},
		fr: {
			label: 'Français',
			lang: 'fr',
			link: '/fr/',
			description:
				'Gestionnaire de tunnels SSH avec client SSH embarqué, daemon dans la zone de notification et interface à la demande.',
			themeConfig: {
				nav: frenchNav(),
				sidebar: frenchSidebar(),
				editLink: {
					pattern: `${REPOSITORY_URL}/edit/main/docs/:path`,
					text: 'Modifier cette page sur GitHub',
				},
				docFooter: { prev: 'Page précédente', next: 'Page suivante' },
				outline: { label: 'Sur cette page' },
				darkModeSwitchLabel: 'Apparence',
				lightModeSwitchTitle: 'Passer au thème clair',
				darkModeSwitchTitle: 'Passer au thème sombre',
				sidebarMenuLabel: 'Menu',
				returnToTopLabel: 'Retour en haut',
				langMenuLabel: 'Changer de langue',
				skipToContentLabel: 'Aller au contenu',
				notFound: {
					title: 'Page introuvable',
					quote: "Cette page n'existe pas ou a été déplacée.",
					linkLabel: "Retour à l'accueil",
					linkText: "Retour à l'accueil",
					code: '404',
				},
			},
		},
	},
})
