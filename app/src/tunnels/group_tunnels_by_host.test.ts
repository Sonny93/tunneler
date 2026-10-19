import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { groupTunnelsByHost } from '#/tunnels/group_tunnels_by_host';

describe('groupTunnelsByHost', () => {
	it('should group tunnels under their host, hosts sorted', () => {
		const groups = groupTunnelsByHost([
			buildTunnel({ id: '1', host: 'zeta' }),
			buildTunnel({ id: '2', host: 'Alpha' }),
			buildTunnel({ id: '3', host: 'zeta' }),
		]);

		expect(groups.map(({ host, tunnels }) => [host, tunnels.length])).toEqual([
			['Alpha', 1],
			['zeta', 2],
		]);
	});

	it('should sort the tunnels of a host by listen port', () => {
		const [group] = groupTunnelsByHost([
			buildTunnel({ id: '1', listenPort: 8080 }),
			buildTunnel({ id: '2', listenPort: 3000 }),
			buildTunnel({ id: '3', listenPort: 5432 }),
		]);

		expect(group?.tunnels.map(({ listenPort }) => listenPort)).toEqual([
			3000, 5432, 8080,
		]);
	});

	it('should return no group without tunnels', () => {
		expect(groupTunnelsByHost([])).toEqual([]);
	});
});
