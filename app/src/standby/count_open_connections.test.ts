import { describe, expect, it } from 'vitest';

import { countOpenConnections } from '#/standby/count_open_connections';
import { buildMetrics, buildTunnel } from '#/testing/factories';

describe('countOpenConnections', () => {
	it('should add up the open connections of every tunnel', () => {
		const tunnels = [
			buildTunnel({ metrics: buildMetrics({ activeConnections: 2 }) }),
			buildTunnel({ metrics: buildMetrics({ activeConnections: 3 }) }),
		];

		expect(countOpenConnections(tunnels)).toBe(5);
	});

	it('should count nothing when there is no tunnel', () => {
		expect(countOpenConnections([])).toBe(0);
	});
});
