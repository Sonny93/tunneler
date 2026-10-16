import { describe, expect, it } from 'vitest';

import { formatBytes } from '#/shared/format_bytes';

describe('formatBytes', () => {
	it.each([
		[0, '0 o'],
		[999, '999 o'],
		[1024, '1 Ko'],
		[1536, '1,5 Ko'],
		[5 * 1024 * 1024, '5 Mo'],
		[3 * 1024 ** 3, '3 Go'],
	])('should format %d bytes as %s', (bytes, expected) => {
		expect(formatBytes(bytes)).toBe(expected);
	});
});
