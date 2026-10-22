import { describe, expect, it } from 'vitest';

import { formatBytes } from '#/shared/format_bytes';

describe('formatBytes', () => {
	it.each([
		[0, '0 B'],
		[999, '999 B'],
		[1024, '1 KB'],
		[1536, '1.5 KB'],
		[5 * 1024 * 1024, '5 MB'],
		[3 * 1024 ** 3, '3 GB'],
	])('should format %d bytes as %s', (bytes, expected) => {
		expect(formatBytes(bytes)).toBe(expected);
	});
});
