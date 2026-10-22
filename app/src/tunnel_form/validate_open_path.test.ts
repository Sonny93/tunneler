import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { validateOpenPath } from '#/tunnel_form/validate_open_path';

function describeError(path: string): string | null {
	const problem = validateOpenPath(path);
	return problem === null ? null : i18n._(problem);
}

describe('validateOpenPath', () => {
	it.each(['', '  ', '/', '/admin?tab=1#top'])('should accept "%s"', (path) => {
		expect(validateOpenPath(path)).toBeNull();
	});

	it('should require a leading slash', () => {
		expect(describeError('admin')).toBe('The path must start with "/".');
	});

	it('should refuse a double leading slash', () => {
		expect(describeError('//evil.example')).toBe(
			'The path must not start with "//".'
		);
	});

	it('should refuse whitespace inside the path', () => {
		expect(describeError('/my page')).toBe(
			'The path must not contain spaces or control characters.'
		);
	});

	it('should refuse a control character', () => {
		expect(describeError('/a\u0007b')).toBe(
			'The path must not contain spaces or control characters.'
		);
	});

	it('should ignore the whitespace around the path', () => {
		expect(validateOpenPath(' /admin ')).toBeNull();
	});
});
