import { describe, expect, it } from 'vitest';

import { getContrastRatio, getRelativeLuminance } from './contrast_ratio';

describe('getContrastRatio', () => {
	it('should return 21 for black on white', () => {
		expect(getContrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 5);
	});

	it('should return 1 for identical colors', () => {
		expect(getContrastRatio('#336699', '#336699')).toBeCloseTo(1, 5);
	});

	it('should return about 4.48 for #777777 on white', () => {
		expect(getContrastRatio('#777777', '#ffffff')).toBeCloseTo(4.48, 2);
	});

	it('should not depend on the argument order', () => {
		expect(getContrastRatio('#ffffff', '#777777')).toBe(
			getContrastRatio('#777777', '#ffffff')
		);
	});

	it('should accept uppercase hex digits', () => {
		expect(getContrastRatio('#FFFFFF', '#000000')).toBeCloseTo(21, 5);
	});
});

describe('getRelativeLuminance', () => {
	it('should be 0 for black and 1 for white', () => {
		expect([
			getRelativeLuminance('#000000'),
			getRelativeLuminance('#ffffff'),
		]).toEqual([0, 1]);
	});

	it('should reject a color that is not #rrggbb', () => {
		expect(() => getRelativeLuminance('red')).toThrow('#rrggbb');
	});
});
