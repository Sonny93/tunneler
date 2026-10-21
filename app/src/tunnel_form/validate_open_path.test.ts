import { describe, expect, it } from 'vitest';

import { validateOpenPath } from '#/tunnel_form/validate_open_path';

describe('validateOpenPath', () => {
	it.each(['', '  ', '/', '/admin?tab=1#top'])('should accept "%s"', (path) => {
		expect(validateOpenPath(path)).toBeNull();
	});

	it('should require a leading slash', () => {
		expect(validateOpenPath('admin')).toBe(
			'Le chemin doit commencer par « / ».'
		);
	});

	it('should refuse a double leading slash', () => {
		expect(validateOpenPath('//evil.example')).toBe(
			'Le chemin ne doit pas commencer par « // ».'
		);
	});

	it('should refuse whitespace inside the path', () => {
		expect(validateOpenPath('/my page')).toBe(
			'Le chemin ne doit contenir ni espace ni caractère de contrôle.'
		);
	});

	it('should refuse a control character', () => {
		expect(validateOpenPath('/a\u0007b')).toBe(
			'Le chemin ne doit contenir ni espace ni caractère de contrôle.'
		);
	});

	it('should ignore the whitespace around the path', () => {
		expect(validateOpenPath(' /admin ')).toBeNull();
	});
});
