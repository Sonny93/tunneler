import { Button, Tooltip, ThemeToggle, useThemeStore } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { QuitButton } from '#/quit/quit_button';
import { getThemeAriaLabel } from '#/header/theme_aria_label';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';

export function HeaderActions() {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openImport = useUiStore((state) => state.openImport);
	const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
	const openSettings = useUiStore((state) => state.openSettings);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const theme = useThemeStore((state) => state.theme);

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
			<TooltipIconButton
				icon="i-mdi-console-line"
				aria-label="Palette de commandes"
				aria-keyshortcuts="Control+K"
				shortcutHint="Ctrl+K"
				variant="ghost"
				color="neutral"
				onClick={handleOpenPalette}
			/>
			<TooltipIconButton
				icon="i-mdi-cog-outline"
				aria-label="Paramètres"
				shortcutHint=","
				variant="ghost"
				color="neutral"
				onClick={openSettings}
			/>
			{/* View transitions crash the WebKitGTK 2.54 web process */}
			<Tooltip content={getThemeAriaLabel(theme)}>
				<ThemeToggle variant="ghost" isTransitionEnabled={false} />
			</Tooltip>
			<TooltipIconButton
				icon="i-mdi-keyboard-outline"
				aria-label="Raccourcis clavier"
				shortcutHint="?"
				variant="ghost"
				color="neutral"
				onClick={handleOpenHelp}
			/>
			<QuitButton />
		</>
	);
}
