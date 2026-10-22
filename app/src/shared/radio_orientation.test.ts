import { describe, expect, it } from 'vitest';

import { getRadioOrientation } from '#/shared/radio_orientation';

describe('getRadioOrientation', () => {
	it.each([1, 2])('should be horizontal with %i option(s)', (optionCount) => {
		expect(getRadioOrientation(optionCount)).toBe('horizontal');
	});

	it.each([3, 4])('should be vertical with %i options', (optionCount) => {
		expect(getRadioOrientation(optionCount)).toBe('vertical');
	});
});
