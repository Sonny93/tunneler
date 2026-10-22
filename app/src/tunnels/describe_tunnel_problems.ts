import type { I18n } from '@lingui/core';

import type { TunnelDto } from '#/daemon/protocol';
import { getTargetHealthError } from '#/tunnels/target_health_label';
import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';

const PROBLEM_SEPARATOR = '. ';

/**
 * The state detail (failure reason, external owner, port conflict) and the
 * unreachable-target error spoken after the tunnel name, or `null` when the
 * tunnel has no problem to explain.
 */
export function describeTunnelProblems(
	tunnel: Readonly<Pick<TunnelDto, 'state' | 'health'>>,
	i18n: I18n
): string | null {
	const problems = [
		getTunnelStateDetail(tunnel.state, i18n),
		getTargetHealthError(tunnel.health),
	].filter((problem) => problem !== null);
	return problems.length === 0 ? null : problems.join(PROBLEM_SEPARATOR);
}
