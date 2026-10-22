import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { TargetHealthDto } from '#/daemon/protocol';

export function getTargetHealthLabel(
	health: Readonly<TargetHealthDto>,
	i18n: I18n
): string {
	switch (health.kind) {
		case 'unknown':
			return t(i18n)`Health unknown`;
		case 'reachable':
			return t(i18n)`Target reachable`;
		case 'unreachable':
			return t(i18n)`Target unreachable`;
	}
}

/** The last error of an unreachable target, `null` for the other health kinds. */
export function getTargetHealthError(
	health: Readonly<TargetHealthDto>
): string | null {
	return health.kind === 'unreachable' ? health.error : null;
}
