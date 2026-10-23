import { describe, expect, it } from 'vitest';

import { areRecordsEqual } from '#/shared/are_records_equal';

describe('areRecordsEqual', () => {
	it('should be true for records with the same values', () => {
		expect(areRecordsEqual({ a: 'x', b: true }, { b: true, a: 'x' })).toBe(
			true
		);
	});

	it('should be false when a value differs', () => {
		expect(areRecordsEqual({ a: 'x' }, { a: 'y' })).toBe(false);
	});

	it('should be false when a key is missing', () => {
		expect(areRecordsEqual({ a: 'x', b: 'y' }, { a: 'x' })).toBe(false);
	});
});
