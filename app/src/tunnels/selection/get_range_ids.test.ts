import { describe, expect, it } from 'vitest';

import { getRangeIds } from '#/tunnels/selection/get_range_ids';

const ORDERED_IDS = ['a', 'b', 'c', 'd'];

describe('getRangeIds', () => {
	it('should include both ends going down', () => {
		expect(getRangeIds(ORDERED_IDS, 'a', 'c')).toEqual(['a', 'b', 'c']);
	});

	it('should keep the list order going up', () => {
		expect(getRangeIds(ORDERED_IDS, 'd', 'b')).toEqual(['b', 'c', 'd']);
	});

	it('should return the target alone when there is no anchor', () => {
		expect(getRangeIds(ORDERED_IDS, null, 'c')).toEqual(['c']);
	});

	it('should return the target alone when the anchor is not listed', () => {
		expect(getRangeIds(ORDERED_IDS, 'gone', 'c')).toEqual(['c']);
	});

	it('should return the target alone when it is the anchor', () => {
		expect(getRangeIds(ORDERED_IDS, 'b', 'b')).toEqual(['b']);
	});
});
