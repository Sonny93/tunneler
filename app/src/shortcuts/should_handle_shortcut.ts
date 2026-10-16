import { isEditableTarget } from '#/shortcuts/is_editable_target';

type ShortcutEvent = Pick<
	KeyboardEvent,
	'ctrlKey' | 'altKey' | 'metaKey' | 'defaultPrevented' | 'target'
>;

/** Whether a key press may trigger a global shortcut in the current context. */
export function shouldHandleShortcut(
	event: Readonly<ShortcutEvent>,
	areGlobalHotkeysEnabled: boolean
): boolean {
	if (!areGlobalHotkeysEnabled || event.defaultPrevented) return false;
	if (event.ctrlKey || event.altKey || event.metaKey) return false;
	return !isEditableTarget(event.target);
}
