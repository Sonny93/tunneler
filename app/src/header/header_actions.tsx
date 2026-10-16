import { Button, IconButton, ThemeToggle } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { QuitButton } from '#/quit/quit_button';
import { getThemeAriaLabel } from '#/header/theme_aria_label';

export function HeaderActions() {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openSettings = useUiStore((state) => state.openSettings);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const handleNewTunnel = () => openTunnelEditor(null);
	const handleOpenHelp = () => setHelpOpen(true);

	return (
		<>
			<Button startIcon="i-mdi-plus" onClick={handleNewTunnel}>
				Nouveau tunnel
			</Button>
			<IconButton
				icon="i-mdi-cog-outline"
				aria-label="Paramètres"
				variant="ghost"
				color="neutral"
				onClick={openSettings}
			/>
			<ThemeToggle variant="ghost" getAriaLabel={getThemeAriaLabel} />
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
