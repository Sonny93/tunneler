import { describe, expect, it } from 'vitest';

import { isEditableTarget } from '#/shortcuts/is_editable_target';

function createInput(type: string): HTMLInputElement {
	const input = document.createElement('input');
	input.type = type;
	return input;
}

describe('isEditableTarget', () => {
	it('should return true when the target is a text input', () => {
		expect(isEditableTarget(createInput('text'))).toBe(true);
	});

	it('should return true when the target is a textarea', () => {
		expect(isEditableTarget(document.createElement('textarea'))).toBe(true);
	});

	it('should return false when the target is a checkbox', () => {
		expect(isEditableTarget(createInput('checkbox'))).toBe(false);
	});

	it('should return false when the target is a radio', () => {
		expect(isEditableTarget(createInput('radio'))).toBe(false);
	});

	it('should return false when the target is not an element', () => {
		expect(isEditableTarget(null)).toBe(false);
	});
});
