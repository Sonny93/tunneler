import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { TargetHealthDto } from '#/daemon/protocol';

export function getTargetHealthLabel(
	health: Readonly<TargetHealthDto>,
	i18n: I18n
): string {
	switch (health.kind) {
		case 'unknown':
			return i18n._(msg`Health unknown`);
		case 'reachable':
			return i18n._(msg`Target reachable`);
		case 'unreachable':
			return i18n._(msg`Target unreachable`);
	}
}

/** The last error of an unreachable target, `null` for the other health kinds. */
export function getTargetHealthError(
	health: Readonly<TargetHealthDto>
): string | null {
	return health.kind === 'unreachable' ? health.error : null;
}
