import { describe, expect, it } from 'vitest';

import {
	COLON_TAG_MESSAGE,
	DIGITS_TAG_MESSAGE,
} from '#/shared/reserved_tag_problem';
import {
	EMPTY_TAG_MESSAGE,
	SEPARATOR_TAG_MESSAGE,
	UNCHANGED_TAG_MESSAGE,
	WHITESPACE_TAG_MESSAGE,
	validateTagName,
} from '#/tag_rename/validate_tag_name';

describe('validateTagName', () => {
	it.each(['', '   '])('should reject the empty name "%s"', (text) => {
		expect(validateTagName(text, 'dev')).toBe(EMPTY_TAG_MESSAGE);
	});

	it('should reject a comma', () => {
		expect(validateTagName('a,b', 'dev')).toBe(SEPARATOR_TAG_MESSAGE);
	});

	it('should reject inner whitespace', () => {
		expect(validateTagName('my tag', 'dev')).toBe(WHITESPACE_TAG_MESSAGE);
	});

	it('should reject a colon', () => {
		expect(validateTagName('nova:3333', 'dev')).toBe(COLON_TAG_MESSAGE);
	});

	it('should reject a name made only of digits', () => {
		expect(validateTagName('3333', 'dev')).toBe(DIGITS_TAG_MESSAGE);
	});

	it('should reject the current name', () => {
		expect(validateTagName(' dev ', 'dev')).toBe(UNCHANGED_TAG_MESSAGE);
	});

	it('should accept a new name', () => {
		expect(validateTagName(' staging ', 'dev')).toBeNull();
	});
});
