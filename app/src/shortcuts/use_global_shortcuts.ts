import { useEffect } from 'react';
import { useGlobalHotkeysStore } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { shouldHandleShortcut } from '#/shortcuts/should_handle_shortcut';
import type { GlobalShortcutKey } from '#/shortcuts/shortcut_definitions';

const SHORTCUT_ACTIONS: Readonly<Record<GlobalShortcutKey, () => void>> = {
	n: () => useUiStore.getState().openTunnelEditor(null),
	',': () => useUiStore.getState().openSettings(),
	'?': () => useUiStore.getState().setHelpOpen(true),
};

function isGlobalShortcutKey(key: string): key is GlobalShortcutKey {
	return Object.hasOwn(SHORTCUT_ACTIONS, key);
}

/** Binds `n`, `,` and `?` on the document. */
export function useGlobalShortcuts(): void {
	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (!isGlobalShortcutKey(event.key)) return;
			const { globalHotkeysEnabled } = useGlobalHotkeysStore.getState();
			if (!shouldHandleShortcut(event, globalHotkeysEnabled)) return;
			event.preventDefault();
			SHORTCUT_ACTIONS[event.key]();
		};
		document.addEventListener('keydown', handleKeyDown);
		return () => document.removeEventListener('keydown', handleKeyDown);
	}, []);
}
