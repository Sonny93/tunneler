import { Button, IconButton, ThemeToggle } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { QuitButton } from '#/quit/quit_button';
import { formatShortcutTooltip } from '#/shared/shortcut_tooltip';

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
				variant="ghost"
				color="neutral"
				onClick={openImport}
			>
				Importer
			</Button>
			<IconButton
				icon="i-mdi-console-line"
				aria-label="Palette de commandes"
				tooltip={{
					content: formatShortcutTooltip('Palette de commandes', 'Ctrl+K'),
				}}
				aria-keyshortcuts="Control+K"
				variant="ghost"
				color="neutral"
				onClick={handleOpenPalette}
			/>
			<IconButton
				icon="i-mdi-cog-outline"
				aria-label="Paramètres"
				tooltip={{ content: formatShortcutTooltip('Paramètres', ',') }}
				variant="ghost"
				color="neutral"
				onClick={openSettings}
			/>
			{/* View transitions crash the WebKitGTK 2.54 web process */}
			<ThemeToggle variant="ghost" isTransitionEnabled={false} tooltip />
			<IconButton
				icon="i-mdi-keyboard-outline"
				aria-label="Raccourcis clavier"
				tooltip={{ content: formatShortcutTooltip('Raccourcis clavier', '?') }}
				variant="ghost"
				color="neutral"
				onClick={handleOpenHelp}
			/>
			<QuitButton />
		</>
	);
}
