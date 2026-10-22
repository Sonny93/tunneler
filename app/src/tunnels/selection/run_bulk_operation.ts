import type { I18n } from '@lingui/core';

import type { TunnelDto } from '#/daemon/protocol';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

export type BulkFailure = { label: string; message: string };

export type BulkOutcome = {
	total: number;
	succeededIds: readonly string[];
	failures: readonly BulkFailure[];
};

/** Runs `perform` on every target at once; a failing tunnel never stops the others. */
export async function runBulkOperation(
	targets: readonly TunnelDto[],
	perform: (tunnel: TunnelDto) => Promise<unknown>,
	i18n: I18n
): Promise<BulkOutcome> {
	const settlements = await Promise.allSettled(targets.map(perform));
	const succeededIds: string[] = [];
	const failures: BulkFailure[] = [];
	settlements.forEach((settlement, index) => {
		const target = targets[index];
		if (settlement.status === 'fulfilled') {
			succeededIds.push(target.id);
			return;
		}
		failures.push({
			label: getTunnelEndpointLabel(target),
			message: describeDaemonError(
				toDaemonCommandError(settlement.reason),
				i18n
			),
		});
	});
	return { total: targets.length, succeededIds, failures };
}
