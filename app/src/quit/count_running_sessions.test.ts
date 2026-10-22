import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { countRunningSessions } from '#/quit/count_running_sessions';

describe('countRunningSessions', () => {
	it('should count zero when nothing runs', () => {
		expect(countRunningSessions([buildTunnel()], [])).toBe(0);
	});

	it('should count the tunnels that hold their port, standby included', () => {
		const tunnels = [
			buildTunnel({ id: 'a', state: { kind: 'active' } }),
			buildTunnel({ id: 'b', state: { kind: 'standby' } }),
			buildTunnel({ id: 'c', state: { kind: 'starting' } }),
			buildTunnel({ id: 'd', state: { kind: 'stopped' } }),
		];

		expect(countRunningSessions(tunnels, [])).toBe(3);
	});

	it('should count the X11 anchors that are not failed', () => {
		const anchors = [
			{ host: 'a', state: { kind: 'active' as const, display: ':10' } },
			{ host: 'b', state: { kind: 'failed' as const, message: 'refusé' } },
		];

		expect(countRunningSessions([], anchors)).toBe(1);
	});
});
