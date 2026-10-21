import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';

describe('countActiveTunnels', () => {
	it('should count running tunnels against the total', () => {
		const tunnels = [
			buildTunnel({ id: 'a', state: { kind: 'active' } }),
			buildTunnel({ id: 'b', state: { kind: 'starting' } }),
			buildTunnel({ id: 'c', state: { kind: 'stopped' } }),
		];

		expect(countActiveTunnels(tunnels)).toEqual({ running: 2, total: 3 });
	});

	it('should not count a tunnel in standby as active', () => {
		const tunnels = [
			buildTunnel({ id: 'a', state: { kind: 'active' } }),
			buildTunnel({ id: 'b', state: { kind: 'standby' } }),
		];

		expect(countActiveTunnels(tunnels)).toEqual({ running: 1, total: 2 });
	});

	it('should return zeros for no tunnels', () => {
		expect(countActiveTunnels([])).toEqual({ running: 0, total: 0 });
	});
});
