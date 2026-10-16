import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { filterTunnels } from '#/tunnels/filter_tunnels';

const TUNNELS = [
	buildTunnel({ id: '1', host: 'nova', tags: ['dev'] }),
	buildTunnel({ id: '2', host: 'zeta', tags: ['dev', 'db'] }),
	buildTunnel({ id: '3', host: 'zeta', tags: [] }),
];

const idsOf = (tunnels: readonly { id: string }[]) =>
	tunnels.map(({ id }) => id);

describe('filterTunnels', () => {
	it('should keep every tunnel for the all filter', () => {
		expect(idsOf(filterTunnels(TUNNELS, { kind: 'all' }))).toEqual([
			'1',
			'2',
			'3',
		]);
	});

	it('should keep the tunnels carrying the tag', () => {
		expect(idsOf(filterTunnels(TUNNELS, { kind: 'tag', tag: 'dev' }))).toEqual([
			'1',
			'2',
		]);
	});

	it('should keep the tunnels of the host', () => {
		expect(
			idsOf(filterTunnels(TUNNELS, { kind: 'host', host: 'zeta' }))
		).toEqual(['2', '3']);
	});
});
