import { describe, expect, it } from 'vitest';

import { moveActiveIndex } from '#/palette/move_active_index';

describe('moveActiveIndex', () => {
	it('should step forward', () => {
		expect(moveActiveIndex(0, 3, 'next')).toBe(1);
	});

	it('should wrap forward from the last option', () => {
		expect(moveActiveIndex(2, 3, 'next')).toBe(0);
	});

	it('should step backward', () => {
		expect(moveActiveIndex(2, 3, 'previous')).toBe(1);
	});

	it('should wrap backward from the first option', () => {
		expect(moveActiveIndex(0, 3, 'previous')).toBe(2);
	});

	it('should jump to the first and last options', () => {
		expect(moveActiveIndex(1, 3, 'first')).toBe(0);
		expect(moveActiveIndex(1, 3, 'last')).toBe(2);
	});

	it('should have no active option without options', () => {
		expect(moveActiveIndex(0, 0, 'next')).toBeNull();
	});
});
