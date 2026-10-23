import { describe, expect, it } from 'vitest';

import { summarizeHosts } from '#/sidebar/summarize_hosts';
import { buildTunnel } from '#/testing/factories';

describe('summarizeHosts', () => {
	it('should count running and total tunnels per host, sorted', () => {
		const summaries = summarizeHosts([
			buildTunnel({ id: '1', host: 'zeta', state: { kind: 'active' } }),
			buildTunnel({ id: '2', host: 'nova' }),
			buildTunnel({ id: '3', host: 'zeta' }),
		]);

		expect(summaries).toEqual([
			{ host: 'nova', running: 0, total: 1 },
			{ host: 'zeta', running: 1, total: 2 },
		]);
	});

	it('should return nothing without tunnels', () => {
		expect(summarizeHosts([])).toEqual([]);
	});
});
