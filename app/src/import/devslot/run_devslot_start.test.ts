import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { TunnelSpecDto } from '#/daemon/protocol';
import {
	createAndStartDevslotTunnels,
	describeDevslotStartFailure,
	startTemporaryDevslotTunnels,
} from '#/import/devslot/run_devslot_start';
import { buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';

const buildItem = (key: string, listenPort: number) => ({
	key,
	spec: {
		host: 'vega',
		kind: 'local',
		listenPort,
		tags: ['devslot'],
	} satisfies TunnelSpecDto,
});

describe('startTemporaryDevslotTunnels', () => {
	it('should keep going after a failure and report it', async () => {
		const gateway = createFakeGateway();
		gateway.startTemporaryTunnel.mockRejectedValueOnce(
			new DaemonCommandError('port_in_use', 'busy')
		);
		const items = [buildItem('0:0', 20_050), buildItem('0:1', 20_051)];

		const outcome = await startTemporaryDevslotTunnels(gateway, items, i18n);

		expect(outcome.startedCount).toBe(1);
		expect(outcome.doneKeys).toEqual(['0:1']);
		expect(outcome.failures).toMatchObject([
			{ key: '0:0', stage: 'startTemporary', messages: ['busy'] },
		]);
	});
});

describe('createAndStartDevslotTunnels', () => {
	it('should report a start failure apart from a creation failure and never delete a created tunnel', async () => {
		const gateway = createFakeGateway();
		gateway.createTunnel
			.mockRejectedValueOnce(new DaemonCommandError('validation_failed', 'bad'))
			.mockResolvedValueOnce(buildTunnel({ id: 'created-b' }));
		gateway.startTunnel.mockRejectedValueOnce(
			new DaemonCommandError('connect_failed', 'no route')
		);
		const items = [buildItem('0:0', 20_050), buildItem('0:1', 20_051)];

		const outcome = await createAndStartDevslotTunnels(gateway, items, i18n);

		expect(outcome.failures.map(({ stage }) => stage)).toEqual([
			'create',
			'start',
		]);
		expect(outcome.doneKeys).toEqual(['0:1']);
		expect(outcome.startedCount).toBe(0);
		expect(gateway.startTunnel).toHaveBeenCalledWith('created-b');
		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
	});

	it('should start every created tunnel', async () => {
		const gateway = createFakeGateway();

		const outcome = await createAndStartDevslotTunnels(
			gateway,
			[buildItem('0:0', 20_050)],
			i18n
		);

		expect(outcome.startedCount).toBe(1);
		expect(gateway.startTunnel).toHaveBeenCalledWith('created-tunnel');
	});
});

describe('describeDevslotStartFailure', () => {
	it('should say that a created tunnel did not start', () => {
		const text = describeDevslotStartFailure(
			{
				key: '0:0',
				label: '20050 → vega:20050',
				stage: 'start',
				messages: ['no route'],
				englishTexts: ['no route'],
			},
			i18n
		);

		expect(text).toBe(
			'20050 → vega:20050 was created but did not start: no route'
		);
	});
});
