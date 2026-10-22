import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getX11StateDetail, getX11StateLabel } from '#/x11/x11_state_label';

describe('getX11StateLabel', () => {
	it.each([
		[{ kind: 'stopped' }, 'Stopped'],
		[{ kind: 'starting' }, 'Starting…'],
		[{ kind: 'active', display: 'localhost:10.0' }, 'Active'],
		[
			{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
			'Reconnecting (attempt 2)',
		],
		[{ kind: 'failed', message: 'refused' }, 'Failed'],
	] as const)('should label %j as %s', (state, label) => {
		expect(getX11StateLabel(state, i18n)).toBe(label);
	});
});

describe('getX11StateDetail', () => {
	it.each([
		[{ kind: 'active', display: 'localhost:10.0' }, 'localhost:10.0'],
		[{ kind: 'failed', message: 'refused' }, 'refused'],
		[{ kind: 'starting' }, null],
		[{ kind: 'stopped' }, null],
		[{ kind: 'reconnecting', attempt: 1, nextRetryAt: 0 }, null],
	] as const)('should give the detail of %j', (state, detail) => {
		expect(getX11StateDetail(state)).toBe(detail);
	});
});
