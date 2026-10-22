import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';

describe('getTunnelStateLabel', () => {
	it.each([
		[{ kind: 'stopped' }, 'Stopped'],
		[{ kind: 'standby' }, 'Standby'],
		[{ kind: 'starting' }, 'Starting…'],
		[{ kind: 'active' }, 'Active'],
		[
			{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
			'Reconnecting (attempt 2)',
		],
		[{ kind: 'failed', reason: { kind: 'connect', message: 'x' } }, 'Failed'],
		[{ kind: 'external', owner: { pid: 1, process: 'ssh' } }, 'External'],
		[{ kind: 'conflict' }, 'Conflict'],
	] as const)('should label %j as %s', (state, label) => {
		expect(getTunnelStateLabel(state, i18n)).toBe(label);
	});
});
