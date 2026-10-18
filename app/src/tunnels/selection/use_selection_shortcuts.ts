import type { KeyboardEvent } from 'react';

import { isEditableTarget } from '#/shortcuts/is_editable_target';

type SelectionShortcutActions = {
	onSelectAll: () => void;
	onClear: () => void;
};

function isSelectAllChord(
	event: Readonly<KeyboardEvent<HTMLElement>>
): boolean {
	return (
		(event.ctrlKey || event.metaKey) &&
		!event.altKey &&
		event.key.toLowerCase() === 'a'
	);
}

/**
 * Ctrl/Cmd+A selects every visible tunnel and Escape clears the selection, for
 * keys pressed inside the list. Events that bubble up from a portal (an open
 * menu or modal) or from a text field are left alone, as are keys another
 * handler already used.
 */
export function useSelectionShortcuts({
	onSelectAll,
	onClear,
}: Readonly<SelectionShortcutActions>): (
	event: KeyboardEvent<HTMLElement>
) => void {
	return (event) => {
		const isInsideList =
			event.target instanceof Node &&
			event.currentTarget.contains(event.target);
		if (!isInsideList || event.defaultPrevented) return;
		if (isSelectAllChord(event) && !isEditableTarget(event.target)) {
			event.preventDefault();
			onSelectAll();
			return;
		}
		if (event.key === 'Escape') onClear();
	};
}
