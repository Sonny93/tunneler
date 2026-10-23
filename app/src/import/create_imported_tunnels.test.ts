import { i18n } from '@lingui/core';
import { describe, expect, it, vi } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { createImportedTunnels } from '#/import/create_imported_tunnels';
import { createFakeGateway } from '#/testing/fake_gateway';

const buildImportSpec = (index: number) => ({
	index,
	spec: {
		host: 'nova',
		kind: 'local' as const,
		listenPort: 3000 + index,
		tags: [],
	},
});

describe('createImportedTunnels', () => {
	it('should create the tunnels in order', async () => {
		const gateway = createFakeGateway();

		await createImportedTunnels(
			gateway,
			[buildImportSpec(0), buildImportSpec(2)],
			vi.fn(),
			i18n
		);

		expect(
			gateway.createTunnel.mock.calls.map(([spec]) => spec.listenPort)
		).toEqual([3000, 3002]);
	});

	it('should report the created indexes and the progress', async () => {
		const onProgress = vi.fn();

		const outcome = await createImportedTunnels(
			createFakeGateway(),
			[buildImportSpec(0), buildImportSpec(2)],
			onProgress,
			i18n
		);

		expect(outcome).toEqual({ createdIndexes: [0, 2], failures: [] });
		expect(onProgress.mock.calls).toEqual([[1], [2]]);
	});

	it('should keep going after a failure and report it with its messages', async () => {
		const gateway = createFakeGateway();
		gateway.createTunnel.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalid', ['Port in use'])
		);

		const outcome = await createImportedTunnels(
			gateway,
			[buildImportSpec(0), buildImportSpec(1)],
			vi.fn(),
			i18n
		);

		expect(outcome).toEqual({
			createdIndexes: [1],
			failures: [
				{ index: 0, label: '3000 → nova:3000', messages: ['Port in use'] },
			],
		});
	});
});
