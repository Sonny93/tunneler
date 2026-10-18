import { describe, expect, it } from 'vitest';

import { resolveToolbarIndex } from '#/tunnels/selection/resolve_toolbar_index';

describe('resolveToolbarIndex', () => {
	it('should move right', () => {
		expect(resolveToolbarIndex('ArrowRight', 1, 4)).toBe(2);
	});

	it('should wrap from the last item to the first with ArrowRight', () => {
		expect(resolveToolbarIndex('ArrowRight', 3, 4)).toBe(0);
	});

	it('should move left', () => {
		expect(resolveToolbarIndex('ArrowLeft', 2, 4)).toBe(1);
	});

	it('should wrap from the first item to the last with ArrowLeft', () => {
		expect(resolveToolbarIndex('ArrowLeft', 0, 4)).toBe(3);
	});

	it('should jump to the ends with Home and End', () => {
		expect([
			resolveToolbarIndex('Home', 2, 4),
			resolveToolbarIndex('End', 1, 4),
		]).toEqual([0, 3]);
	});

	it('should ignore other keys', () => {
		expect(resolveToolbarIndex('a', 1, 4)).toBeNull();
	});
});
