import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { TunnelStateDto } from '#/daemon/protocol';

export function getTunnelStateLabel(
	state: Readonly<TunnelStateDto>,
	i18n: I18n
): string {
	switch (state.kind) {
		case 'stopped':
			return i18n._(msg`Stopped`);
		case 'standby':
			return i18n._(msg`Standby`);
		case 'starting':
			return i18n._(msg`Starting…`);
		case 'active':
			return i18n._(msg`Active`);
		case 'reconnecting': {
			const { attempt } = state;
			return i18n._(msg`Reconnecting (attempt ${attempt})`);
		}
		case 'failed':
			return i18n._(msg`Failed`);
		case 'external':
			return i18n._(msg`External`);
		case 'conflict':
			return i18n._(msg`Conflict`);
	}
}
