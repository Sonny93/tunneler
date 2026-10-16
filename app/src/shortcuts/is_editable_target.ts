const EDITABLE_SELECTOR =
	'input, textarea, select, [contenteditable]:not([contenteditable="false"])';

export function isEditableTarget(target: EventTarget | null): boolean {
	return (
		target instanceof Element && target.closest(EDITABLE_SELECTOR) !== null
	);
}
