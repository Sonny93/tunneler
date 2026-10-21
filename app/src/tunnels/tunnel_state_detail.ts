import type { TunnelStateDto } from '#/daemon/protocol';
import { describeProcess } from '#/externals/describe_process';
import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

/** What explains a state beyond its label, or `null` when the label says it all. */
export function getTunnelStateDetail(
	state: Readonly<TunnelStateDto>
): string | null {
	switch (state.kind) {
		case 'failed':
			return getFailureReasonLabel(state.reason);
		case 'standby':
			return state.lastFailure
				? `dernier réveil en échec : ${getFailureReasonLabel(state.lastFailure)}`
				: null;
		case 'external':
			return `fourni par ${describeProcess(state.owner)}`;
		case 'conflict':
			return state.owner
				? `port occupé par ${describeProcess(state.owner)}`
				: 'port occupé par un processus inconnu';
		case 'stopped':
		case 'starting':
		case 'active':
		case 'reconnecting':
			return null;
	}
}
