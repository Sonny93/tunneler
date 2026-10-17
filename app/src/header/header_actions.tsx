import { Button, IconButton, ThemeToggle } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { QuitButton } from '#/quit/quit_button';
import { getThemeAriaLabel } from '#/header/theme_aria_label';

export function HeaderActions() {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openImport = useUiStore((state) => state.openImport);
	const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
	const openSettings = useUiStore((state) => state.openSettings);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const handleNewTunnel = () => openTunnelEditor(null);
	const handleOpenPalette = () => setPaletteOpen(true);
	const handleOpenHelp = () => setHelpOpen(true);

	return (
		<>
			<Button startIcon="i-mdi-plus" onClick={handleNewTunnel}>
				Nouveau tunnel
			</Button>
			<Button
				startIcon="i-mdi-import"
				variant="outline"
				color="neutral"
				onClick={openImport}
			>
				Importer
			</Button>
			<IconButton
				icon="i-mdi-console-line"
				aria-label="Palette de commandes"
				aria-keyshortcuts="Control+K"
				variant="ghost"
				color="neutral"
				onClick={handleOpenPalette}
			/>
			<IconButton
				icon="i-mdi-cog-outline"
				aria-label="Paramètres"
				variant="ghost"
				color="neutral"
				onClick={openSettings}
			/>
			{/* View transitions crash the WebKitGTK 2.54 web process */}
			<ThemeToggle
				variant="ghost"
				getAriaLabel={getThemeAriaLabel}
				isTransitionEnabled={false}
			/>
			<IconButton
				icon="i-mdi-keyboard-outline"
				aria-label="Raccourcis clavier"
				variant="ghost"
				color="neutral"
				onClick={handleOpenHelp}
			/>
			<QuitButton />
		</>
	);
}
