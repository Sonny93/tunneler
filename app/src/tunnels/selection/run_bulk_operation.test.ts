import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { runBulkOperation } from '#/tunnels/selection/run_bulk_operation';

const targets = [
	buildTunnel({ id: 'a', listenPort: 3001 }),
	buildTunnel({ id: 'b', listenPort: 3002 }),
];

describe('runBulkOperation', () => {
	it('should report every tunnel as succeeded when none fails', async () => {
		const outcome = await runBulkOperation(targets, () => Promise.resolve());

		expect(outcome).toEqual({
			total: 2,
			succeededIds: ['a', 'b'],
			failures: [],
		});
	});

	it('should keep going after a failure and report its message', async () => {
		const outcome = await runBulkOperation(targets, ({ id }) =>
			id === 'a' ? Promise.reject(new Error('boom')) : Promise.resolve()
		);

		expect(outcome).toEqual({
			total: 2,
			succeededIds: ['b'],
			failures: [{ label: '3001 → nova:3001', message: 'boom' }],
		});
	});
});
