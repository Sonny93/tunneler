import { useLingui } from '@lingui/react/macro';
import {
	Button,
	IconButton,
	Menu,
	MenuItem,
	ThemeToggle,
} from '@minimalstuff/ui';

import { useOpenDocumentation } from '#/documentation/use_open_documentation';
import { QuitButton } from '#/quit/quit_button';
import { formatShortcutTooltip } from '#/shared/shortcut_tooltip';
import { useUiStore } from '#/state/ui_store';

export function HeaderActions() {
	const { t } = useLingui();
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openTemporaryTunnelEditor = useUiStore(
		(state) => state.openTemporaryTunnelEditor
	);
	const openImport = useUiStore((state) => state.openImport);
	const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
	const openSettings = useUiStore((state) => state.openSettings);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const handleNewTunnel = () => openTunnelEditor(null);
	const handleOpenSettings = () => openSettings();
	const handleOpenPalette = () => setPaletteOpen(true);
	const handleOpenHelp = () => setHelpOpen(true);
	const openDocumentation = useOpenDocumentation();
	const handleOpenDocumentation = () => void openDocumentation('home');

	return (
		<>
			<div className="inline-flex">
				<Button
					startIcon="i-mdi-plus"
					className="relative rounded-r-none focus-visible:z-10"
					onClick={handleNewTunnel}
				>
					{t`New tunnel`}
				</Button>
				<Menu
					align="end"
					trigger={
						<IconButton
							icon="i-mdi-chevron-down"
							aria-label={t`Other ways to create a tunnel`}
							tooltip
							variant="solid"
							color="primary"
							className="relative rounded-l-none border-l border-l-white/40 focus-visible:z-10"
						/>
					}
				>
					<MenuItem icon="i-mdi-timer-sand" onClick={openTemporaryTunnelEditor}>
						{t`Temporary tunnel…`}
					</MenuItem>
				</Menu>
			</div>
			<Button
				startIcon="i-mdi-import"
				variant="ghost"
				color="neutral"
				onClick={openImport}
			>
				{t`Import`}
			</Button>
			<IconButton
				icon="i-mdi-console-line"
				aria-label={t`Command palette`}
				tooltip={{
					content: formatShortcutTooltip(t`Command palette`, 'Ctrl+K'),
				}}
				aria-keyshortcuts="Control+K"
				variant="ghost"
				color="neutral"
				onClick={handleOpenPalette}
			/>
			<IconButton
				icon="i-mdi-cog-outline"
				aria-label={t`Settings`}
				tooltip={{ content: formatShortcutTooltip(t`Settings`, ',') }}
				variant="ghost"
				color="neutral"
				onClick={handleOpenSettings}
			/>
			<ThemeToggle variant="ghost" tooltip />
			<IconButton
				icon="i-mdi-keyboard-outline"
				aria-label={t`Keyboard shortcuts`}
				tooltip={{
					content: formatShortcutTooltip(t`Keyboard shortcuts`, '?'),
				}}
				variant="ghost"
				color="neutral"
				onClick={handleOpenHelp}
			/>
			<IconButton
				icon="i-mdi-help-circle-outline"
				aria-label={t`Documentation`}
				tooltip={{ content: formatShortcutTooltip(t`Documentation`, 'F1') }}
				aria-keyshortcuts="F1"
				variant="ghost"
				color="neutral"
				onClick={handleOpenDocumentation}
			/>
			<QuitButton />
		</>
	);
}
