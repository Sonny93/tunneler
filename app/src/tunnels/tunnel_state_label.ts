import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { TunnelStateDto } from '#/daemon/protocol';

export function getTunnelStateLabel(
	state: Readonly<TunnelStateDto>,
	i18n: I18n
): string {
	switch (state.kind) {
		case 'stopped':
			return t(i18n)`Stopped`;
		case 'standby':
			return t(i18n)`Standby`;
		case 'starting':
			return t(i18n)`Starting…`;
		case 'active':
			return t(i18n)`Active`;
		case 'reconnecting': {
			const { attempt } = state;
			return t(i18n)`Reconnecting (attempt ${attempt})`;
		}
		case 'failed':
			return t(i18n)`Failed`;
		case 'external':
			return t(i18n)`External`;
		case 'conflict':
			return t(i18n)`Conflict`;
	}
}
