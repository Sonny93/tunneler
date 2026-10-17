import { useEffect } from 'react';
import { useGlobalHotkeysStore } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { isPaletteChord } from '#/shortcuts/is_palette_chord';
import { shouldHandleShortcut } from '#/shortcuts/should_handle_shortcut';
import type { GlobalShortcutKey } from '#/shortcuts/shortcut_definitions';

const SHORTCUT_ACTIONS: Readonly<Record<GlobalShortcutKey, () => void>> = {
	n: () => useUiStore.getState().openTunnelEditor(null),
	i: () => useUiStore.getState().openImport(),
	',': () => useUiStore.getState().openSettings(),
	'?': () => useUiStore.getState().setHelpOpen(true),
};

function isGlobalShortcutKey(key: string): key is GlobalShortcutKey {
	return Object.hasOwn(SHORTCUT_ACTIONS, key);
}

function handleKey(event: KeyboardEvent): void {
	if (!isGlobalShortcutKey(event.key)) return;
	const { globalHotkeysEnabled } = useGlobalHotkeysStore.getState();
	if (!shouldHandleShortcut(event, globalHotkeysEnabled)) return;
	event.preventDefault();
	SHORTCUT_ACTIONS[event.key]();
}

/** The palette chord is a modifier chord, so it also works while typing in a field. */
function handlePaletteChord(event: KeyboardEvent): void {
	if (!isPaletteChord(event) || event.defaultPrevented) return;
	const { globalHotkeysEnabled } = useGlobalHotkeysStore.getState();
	if (!globalHotkeysEnabled) return;
	event.preventDefault();
	useUiStore.getState().setPaletteOpen(true);
}

/** Binds `n`, `i`, `,`, `?` and Ctrl/Meta+K on the document. */
export function useGlobalShortcuts(): void {
	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			handlePaletteChord(event);
			handleKey(event);
		};
		document.addEventListener('keydown', handleKeyDown);
		return () => document.removeEventListener('keydown', handleKeyDown);
	}, []);
}
