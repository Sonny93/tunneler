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

	it('should sort the tunnels of a host by name, ignoring case', () => {
		const [group] = groupTunnelsByHost([
			buildTunnel({ id: '1', name: 'beta' }),
			buildTunnel({ id: '2', name: 'Alpha' }),
			buildTunnel({ id: '3', name: 'Gamma' }),
		]);

		expect(group?.tunnels.map(({ name }) => name)).toEqual([
			'Alpha',
			'beta',
			'Gamma',
		]);
	});

	it('should return no group without tunnels', () => {
		expect(groupTunnelsByHost([])).toEqual([]);
	});
});
