import { describe, expect, it } from 'vitest';

import { parsePort } from '#/tunnel_form/parse_port';

describe('parsePort', () => {
	it.each([
		['1', 1],
		['8080', 8080],
		[' 65535 ', 65_535],
	])('should parse "%s" as %i', (text, expected) => {
		expect(parsePort(text)).toBe(expected);
	});

	it.each(['', '0', '65536', '-1', '80.5', 'abc', '1e3'])(
		'should reject "%s"',
		(text) => {
			expect(parsePort(text)).toBeNull();
		}
	);
});
