import { describe, expect, it } from 'vitest';

import { buildX11Announcement } from '#/x11/x11_announcement';

describe('buildX11Announcement', () => {
	it('should include the display of an active anchor', () => {
		expect(
			buildX11Announcement('nova', {
				kind: 'active',
				display: 'localhost:10.0',
			})
		).toBe('X11 nova : actif (localhost:10.0)');
	});

	it('should give only the label when there is no detail', () => {
		expect(buildX11Announcement('nova', { kind: 'stopped' })).toBe(
			'X11 nova : arrêté'
		);
	});
});
