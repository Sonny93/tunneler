const EDITABLE_SELECTOR =
	'input, textarea, select, [contenteditable]:not([contenteditable="false"])';

const NON_TEXT_INPUT_TYPES: ReadonlySet<string> = new Set([
	'checkbox',
	'radio',
	'button',
	'submit',
	'reset',
	'range',
	'color',
	'file',
	'image',
]);

function isTextEntryInput(element: Element): boolean {
	return !(
		element instanceof HTMLInputElement &&
		NON_TEXT_INPUT_TYPES.has(element.type)
	);
}

export function isEditableTarget(target: EventTarget | null): boolean {
	if (!(target instanceof Element)) return false;
	const editableAncestor = target.closest(EDITABLE_SELECTOR);
	return editableAncestor !== null && isTextEntryInput(editableAncestor);
}
