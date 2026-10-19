import { describe, expect, it } from 'vitest';

import { validateTags } from '#/tunnel_form/validate_tags';
import { WHITESPACE_TAG_MESSAGE } from '#/tag_rename/validate_tag_name';
import {
	COLON_TAG_MESSAGE,
	DIGITS_TAG_MESSAGE,
} from '#/shared/reserved_tag_problem';

describe('validateTags', () => {
	it('should accept no tag at all', () => {
		expect(validateTags('')).toBeNull();
	});

	it('should accept several valid tags', () => {
		expect(validateTags('dev, adonis')).toBeNull();
	});

	it('should reject a tag with inner whitespace', () => {
		expect(validateTags('dev, my tag')).toBe(WHITESPACE_TAG_MESSAGE);
	});

	it('should reject a tag containing a colon', () => {
		expect(validateTags('dev, nova:3333')).toBe(COLON_TAG_MESSAGE);
	});

	it('should reject a tag made only of digits', () => {
		expect(validateTags('3333')).toBe(DIGITS_TAG_MESSAGE);
	});
});
