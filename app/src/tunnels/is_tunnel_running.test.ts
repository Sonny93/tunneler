import { describe, expect, it } from 'vitest';

import { isTunnelRunning } from '#/tunnels/is_tunnel_running';

describe('isTunnelRunning', () => {
	it.each([
		[{ kind: 'starting' }, true],
		[{ kind: 'active' }, true],
		[{ kind: 'reconnecting', attempt: 1, nextRetryAt: 0 }, true],
		[{ kind: 'stopped' }, false],
		[{ kind: 'external', owner: { pid: 1, process: 'ssh' } }, false],
		[{ kind: 'conflict' }, false],
		[{ kind: 'failed', reason: { kind: 'connect', message: 'x' } }, false],
	] as const)('should return the running flag of %j', (state, expected) => {
		expect(isTunnelRunning(state)).toBe(expected);
	});
});
