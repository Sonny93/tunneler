import type { TunnelDto } from '#/daemon/protocol';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
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
	perform: (tunnel: TunnelDto) => Promise<unknown>
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
			message: toDaemonCommandError(settlement.reason).message,
		});
	});
	return { total: targets.length, succeededIds, failures };
}
