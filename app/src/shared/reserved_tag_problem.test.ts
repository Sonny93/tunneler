import { describe, expect, it } from 'vitest';

import {
	COLON_TAG_MESSAGE,
	DIGITS_TAG_MESSAGE,
	findReservedTagProblem,
} from '#/shared/reserved_tag_problem';

describe('findReservedTagProblem', () => {
	it('should reject a tag containing a colon', () => {
		expect(findReservedTagProblem('nova:3333')).toBe(COLON_TAG_MESSAGE);
	});

	it('should reject a tag made only of digits', () => {
		expect(findReservedTagProblem('3333')).toBe(DIGITS_TAG_MESSAGE);
	});

	it.each(['dev', 'v2', '3333a', 'a-1'])(
		'should accept the tag "%s"',
		(tag) => {
			expect(findReservedTagProblem(tag)).toBeNull();
		}
	);
});
