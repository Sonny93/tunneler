import { describe, expect, it } from 'vitest';

import { parseTags } from '#/tunnel_form/parse_tags';

describe('parseTags', () => {
	it('should trim tags', () => {
		expect(parseTags(' db ,  web')).toEqual(['db', 'web']);
	});

	it('should drop empty entries', () => {
		expect(parseTags('db,, ,web,')).toEqual(['db', 'web']);
	});

	it('should deduplicate keeping the first occurrence', () => {
		expect(parseTags('db, web, db')).toEqual(['db', 'web']);
	});

	it('should return no tags for blank text', () => {
		expect(parseTags('   ')).toEqual([]);
	});
});
