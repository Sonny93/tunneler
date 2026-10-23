import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { TunnelStateDto } from '#/daemon/protocol';
import { describeProcess } from '#/externals/describe_process';
import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

/** What explains a state beyond its label, or `null` when the label says it all. */
export function getTunnelStateDetail(
	state: Readonly<TunnelStateDto>,
	i18n: I18n
): string | null {
	switch (state.kind) {
		case 'failed':
			return getFailureReasonLabel(state.reason, i18n);
		case 'standby': {
			if (!state.lastFailure) return null;
			const failure = getFailureReasonLabel(state.lastFailure, i18n);
			return i18n._(msg`last wake-up failed: ${failure}`);
		}
		case 'external': {
			const owner = describeProcess(state.owner);
			return i18n._(msg`provided by ${owner}`);
		}
		case 'conflict': {
			if (!state.owner) return i18n._(msg`port in use by an unknown process`);
			const owner = describeProcess(state.owner);
			return i18n._(msg`port in use by ${owner}`);
		}
		case 'stopped':
		case 'starting':
		case 'active':
		case 'reconnecting':
			return null;
	}
}
