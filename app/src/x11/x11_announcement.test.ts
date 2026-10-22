import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildX11Announcement } from '#/x11/x11_announcement';

describe('buildX11Announcement', () => {
	it('should include the display of an active anchor', () => {
		expect(
			buildX11Announcement(
				'nova',
				{ kind: 'active', display: 'localhost:10.0' },
				i18n
			)
		).toBe('X11 nova: active (localhost:10.0)');
	});

	it('should give only the label when there is no detail', () => {
		expect(buildX11Announcement('nova', { kind: 'stopped' }, i18n)).toBe(
			'X11 nova: stopped'
		);
	});
});
