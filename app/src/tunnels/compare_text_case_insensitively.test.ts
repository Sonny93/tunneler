import { describe, expect, it } from 'vitest';

import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

describe('compareTextCaseInsensitively', () => {
	it('should sort regardless of case', () => {
		expect(['b', 'A', 'c'].sort(compareTextCaseInsensitively)).toEqual([
			'A',
			'b',
			'c',
		]);
	});

	it('should consider texts differing only by case as equal', () => {
		expect(compareTextCaseInsensitively('Dev', 'dev')).toBe(0);
	});
});
