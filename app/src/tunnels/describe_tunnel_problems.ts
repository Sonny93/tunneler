import type { TunnelDto } from '#/daemon/protocol';
import { getTargetHealthError } from '#/tunnels/target_health_label';
import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

const PROBLEM_SEPARATOR = '. ';

/**
 * The failure reason and the unreachable-target error spoken after the
 * tunnel name, or `null` when the tunnel has no problem to explain.
 */
export function describeTunnelProblems(
	tunnel: Readonly<Pick<TunnelDto, 'state' | 'health'>>
): string | null {
	const failureReason =
		tunnel.state.kind === 'failed'
			? getFailureReasonLabel(tunnel.state.reason)
			: null;
	const problems = [failureReason, getTargetHealthError(tunnel.health)].filter(
		(problem) => problem !== null
	);
	return problems.length === 0 ? null : problems.join(PROBLEM_SEPARATOR);
}
