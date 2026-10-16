import { describe, expect, it } from 'vitest';

import { shouldHandleShortcut } from '#/shortcuts/should_handle_shortcut';

const plainEvent = {
	ctrlKey: false,
	altKey: false,
	metaKey: false,
	defaultPrevented: false,
	target: document.body,
};

describe('shouldHandleShortcut', () => {
	it('should handle a plain key press on the page', () => {
		expect(shouldHandleShortcut(plainEvent, true)).toBe(true);
	});

	it.each(['ctrlKey', 'altKey', 'metaKey'] as const)(
		'should ignore the key press with %s',
		(modifier) => {
			expect(
				shouldHandleShortcut({ ...plainEvent, [modifier]: true }, true)
			).toBe(false);
		}
	);

	it('should ignore the key press when global hotkeys are disabled', () => {
		expect(shouldHandleShortcut(plainEvent, false)).toBe(false);
	});

	it('should ignore a prevented event', () => {
		expect(
			shouldHandleShortcut({ ...plainEvent, defaultPrevented: true }, true)
		).toBe(false);
	});

	it.each(['input', 'textarea', 'select'])(
		'should ignore a %s target',
		(tag) => {
			const target = document.createElement(tag);

			expect(shouldHandleShortcut({ ...plainEvent, target }, true)).toBe(false);
		}
	);

	it('should ignore a contenteditable target', () => {
		const target = document.createElement('div');
		target.setAttribute('contenteditable', 'true');

		expect(shouldHandleShortcut({ ...plainEvent, target }, true)).toBe(false);
	});

	it('should ignore the child of a contenteditable element', () => {
		const parent = document.createElement('div');
		parent.setAttribute('contenteditable', '');
		const target = document.createElement('span');
		parent.append(target);

		expect(shouldHandleShortcut({ ...plainEvent, target }, true)).toBe(false);
	});
});
