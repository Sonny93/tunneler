import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getX11HeaderStatus } from '#/x11/x11_header_status';

describe('getX11HeaderStatus', () => {
	it.each([
		[{ kind: 'stopped' }, null],
		[{ kind: 'starting' }, 'X11: starting…'],
		[
			{ kind: 'active', display: 'localhost:10.0' },
			'X11 active: localhost:10.0',
		],
		[
			{ kind: 'reconnecting', attempt: 3, nextRetryAt: 0 },
			'X11: reconnecting (attempt 3)',
		],
		[{ kind: 'failed', message: 'refused' }, 'X11: failed'],
	] as const)('should give the status of %j', (state, status) => {
		expect(getX11HeaderStatus(state, i18n)).toBe(status);
	});
});
