import { useEffect } from 'react';
import { useGlobalHotkeysStore } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { isPaletteChord } from '#/shortcuts/is_palette_chord';
import { shouldHandleShortcut } from '#/shortcuts/should_handle_shortcut';
import type { GlobalShortcutKey } from '#/shortcuts/shortcut_definitions';
import { useOpenDocumentation } from '#/documentation/use_open_documentation';

const UI_SHORTCUT_ACTIONS: Readonly<
	Record<Exclude<GlobalShortcutKey, 'F1'>, () => void>
> = {
	n: () => useUiStore.getState().openTunnelEditor(null),
	i: () => useUiStore.getState().openImport(),
	',': () => useUiStore.getState().openSettings(),
	b: () => useUiStore.getState().toggleSidebar(),
	'?': () => useUiStore.getState().setHelpOpen(true),
};

function isGlobalShortcutKey(
	actions: Readonly<Record<string, () => void>>,
	key: string
): key is GlobalShortcutKey {
	return Object.hasOwn(actions, key);
}

function handleKey(
	event: KeyboardEvent,
	actions: Readonly<Record<GlobalShortcutKey, () => void>>
): void {
	if (!isGlobalShortcutKey(actions, event.key)) return;
	const { globalHotkeysEnabled } = useGlobalHotkeysStore.getState();
	if (!shouldHandleShortcut(event, globalHotkeysEnabled)) return;
	event.preventDefault();
	actions[event.key]();
}

/** The palette chord is a modifier chord, so it also works while typing in a field. */
function handlePaletteChord(event: KeyboardEvent): void {
	if (!isPaletteChord(event) || event.defaultPrevented) return;
	const { globalHotkeysEnabled } = useGlobalHotkeysStore.getState();
	if (!globalHotkeysEnabled) return;
	event.preventDefault();
	useUiStore.getState().setPaletteOpen(true);
}

/** Binds `n`, `i`, `,`, `b`, `?`, `F1` and Ctrl/Meta+K on the document. */
export function useGlobalShortcuts(): void {
	const openDocumentation = useOpenDocumentation();

	useEffect(() => {
		const actions = {
			...UI_SHORTCUT_ACTIONS,
			F1: () => void openDocumentation('home'),
		};
		const handleKeyDown = (event: KeyboardEvent) => {
			handlePaletteChord(event);
			handleKey(event, actions);
		};
		document.addEventListener('keydown', handleKeyDown);
		return () => document.removeEventListener('keydown', handleKeyDown);
	}, [openDocumentation]);
}
