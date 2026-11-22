import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import type { TemporaryTunnelSpecDto, TunnelSpecDto } from '#/daemon/protocol';
import type { DevslotStartItem } from '#/import/devslot/build_devslot_specs';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type DevslotFailureStage = 'startTemporary' | 'create' | 'start';

export type DevslotStartFailure = {
	key: string;
	label: string;
	stage: DevslotFailureStage;
	messages: string[];
};

export type DevslotStartOutcome = {
	/** Ports that no longer need to be started: a tunnel exists for them. */
	doneKeys: string[];
	startedCount: number;
	failures: DevslotStartFailure[];
};

function buildFailure(
	item: Readonly<DevslotStartItem<TunnelSpecDto | TemporaryTunnelSpecDto>>,
	stage: DevslotFailureStage,
	rejection: unknown,
	i18n: I18n
): DevslotStartFailure {
	return {
		key: item.key,
		label: getTunnelEndpointLabel(item.spec),
		stage,
		messages: getSubmissionErrorMessages(toDaemonCommandError(rejection), i18n),
	};
}

type ItemResult = {
	key: string;
	isDone: boolean;
	isStarted: boolean;
	failures: DevslotStartFailure[];
};

type ItemRunner<TSpec> = (
	item: Readonly<DevslotStartItem<TSpec>>
) => Promise<ItemResult>;

function foldResults(results: readonly ItemResult[]): DevslotStartOutcome {
	return {
		doneKeys: results.filter(({ isDone }) => isDone).map(({ key }) => key),
		startedCount: results.filter(({ isStarted }) => isStarted).length,
		failures: results.flatMap(({ failures }) => failures),
	};
}

/** Runs the items one after the other; a failure does not stop the next ones. */
async function runSequentially<TSpec>(
	items: readonly DevslotStartItem<TSpec>[],
	runItem: ItemRunner<TSpec>
): Promise<DevslotStartOutcome> {
	const results: ItemResult[] = [];
	for (const item of items) {
		results.push(await runItem(item));
	}
	return foldResults(results);
}

export function startTemporaryDevslotTunnels(
	gateway: DaemonGateway,
	items: readonly DevslotStartItem<TemporaryTunnelSpecDto>[],
	i18n: I18n
): Promise<DevslotStartOutcome> {
	const runItem: ItemRunner<TemporaryTunnelSpecDto> = async (item) => {
		try {
			await gateway.startTemporaryTunnel(item.spec);
			return { key: item.key, isDone: true, isStarted: true, failures: [] };
		} catch (rejection) {
			const failure = buildFailure(item, 'startTemporary', rejection, i18n);
			return {
				key: item.key,
				isDone: false,
				isStarted: false,
				failures: [failure],
			};
		}
	};
	return runSequentially(items, runItem);
}

/** Creates then starts each tunnel; a created tunnel is never rolled back when its start fails. */
export function createAndStartDevslotTunnels(
	gateway: DaemonGateway,
	items: readonly DevslotStartItem<TunnelSpecDto>[],
	i18n: I18n
): Promise<DevslotStartOutcome> {
	const runItem: ItemRunner<TunnelSpecDto> = async (item) => {
		let createdId: string;
		try {
			createdId = (await gateway.createTunnel(item.spec)).id;
		} catch (rejection) {
			const failure = buildFailure(item, 'create', rejection, i18n);
			return {
				key: item.key,
				isDone: false,
				isStarted: false,
				failures: [failure],
			};
		}
		try {
			await gateway.startTunnel(createdId);
			return { key: item.key, isDone: true, isStarted: true, failures: [] };
		} catch (rejection) {
			const failure = buildFailure(item, 'start', rejection, i18n);
			return {
				key: item.key,
				isDone: true,
				isStarted: false,
				failures: [failure],
			};
		}
	};
	return runSequentially(items, runItem);
}

export function describeDevslotStartFailure(
	{ label, stage, messages }: Readonly<DevslotStartFailure>,
	i18n: I18n
): string {
	const messageList = messages.join(', ');
	switch (stage) {
		case 'create':
			return i18n._(msg`Could not create ${label}: ${messageList}`);
		case 'start':
			return i18n._(
				msg`${label} was created but did not start: ${messageList}`
			);
		case 'startTemporary':
			return i18n._(msg`Could not start ${label}: ${messageList}`);
	}
}
