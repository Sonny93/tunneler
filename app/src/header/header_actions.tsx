import {
	Button,
	IconButton,
	Menu,
	MenuItem,
	ThemeToggle,
} from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { QuitButton } from '#/quit/quit_button';
import { formatShortcutTooltip } from '#/shared/shortcut_tooltip';

export function HeaderActions() {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openTemporaryTunnelEditor = useUiStore(
		(state) => state.openTemporaryTunnelEditor
	);
	const openImport = useUiStore((state) => state.openImport);
	const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
	const openSettings = useUiStore((state) => state.openSettings);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const handleNewTunnel = () => openTunnelEditor(null);
	const handleOpenPalette = () => setPaletteOpen(true);
	const handleOpenHelp = () => setHelpOpen(true);

	return (
		<>
			<div className="inline-flex">
				<Button
					startIcon="i-mdi-plus"
					className="relative rounded-r-none focus-visible:z-10"
					onClick={handleNewTunnel}
				>
					Nouveau tunnel
				</Button>
				<Menu
					align="end"
					trigger={
						<IconButton
							icon="i-mdi-chevron-down"
							aria-label="Autres façons de créer un tunnel"
							tooltip
							className="relative rounded-l-none border-l border-l-white/40 focus-visible:z-10"
						/>
					}
				>
					<MenuItem icon="i-mdi-timer-sand" onClick={openTemporaryTunnelEditor}>
						Tunnel temporaire…
					</MenuItem>
				</Menu>
			</div>
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
			<ThemeToggle variant="ghost" tooltip />
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
