import { describe, expect, it } from 'vitest';

import { isPaletteChord } from '#/shortcuts/is_palette_chord';

const NO_MODIFIER = {
	ctrlKey: false,
	metaKey: false,
	altKey: false,
	shiftKey: false,
};

describe('isPaletteChord', () => {
	it('should accept Ctrl+K', () => {
		expect(isPaletteChord({ ...NO_MODIFIER, key: 'k', ctrlKey: true })).toBe(
			true
		);
	});

	it('should accept Meta+K', () => {
		expect(isPaletteChord({ ...NO_MODIFIER, key: 'k', metaKey: true })).toBe(
			true
		);
	});

	it('should accept an uppercase K', () => {
		expect(isPaletteChord({ ...NO_MODIFIER, key: 'K', ctrlKey: true })).toBe(
			true
		);
	});

	it('should reject K alone', () => {
		expect(isPaletteChord({ ...NO_MODIFIER, key: 'k' })).toBe(false);
	});

	it('should reject another key', () => {
		expect(isPaletteChord({ ...NO_MODIFIER, key: 'n', ctrlKey: true })).toBe(
			false
		);
	});

	it.each(['altKey', 'shiftKey'] as const)(
		'should reject %s on top',
		(modifier) => {
			expect(
				isPaletteChord({
					...NO_MODIFIER,
					key: 'k',
					ctrlKey: true,
					[modifier]: true,
				})
			).toBe(false);
		}
	);
});
