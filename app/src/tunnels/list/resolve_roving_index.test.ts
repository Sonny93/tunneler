import { describe, expect, it } from 'vitest';

import {
	resolveRovingColumnIndex,
	resolveRovingIndex,
} from '#/tunnels/list/resolve_roving_index';

describe('resolveRovingIndex', () => {
	it.each([
		['ArrowDown', 1, 3, 2],
		['ArrowDown', 2, 3, 2],
		['ArrowUp', 1, 3, 0],
		['ArrowUp', 0, 3, 0],
		['Home', 2, 3, 0],
		['End', 0, 3, 2],
	])('should map %s from %i of %i to %i', (key, current, count, expected) => {
		expect(resolveRovingIndex(key, current, count)).toBe(expected);
	});

	it('should ignore other keys', () => {
		expect(resolveRovingIndex('a', 0, 3)).toBeNull();
	});
});

describe('resolveRovingColumnIndex', () => {
	it.each([
		['ArrowRight', 0, 3, 1],
		['ArrowRight', 2, 3, 2],
		['ArrowLeft', 2, 3, 1],
		['ArrowLeft', 0, 3, 0],
	])('should map %s from %i of %i to %i', (key, current, count, expected) => {
		expect(resolveRovingColumnIndex(key, current, count)).toBe(expected);
	});

	it('should ignore vertical keys', () => {
		expect(resolveRovingColumnIndex('ArrowDown', 0, 3)).toBeNull();
	});
});
