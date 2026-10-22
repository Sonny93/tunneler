import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { ActiveCount } from '#/tunnels/active_count';

/** Spoken form, such as `2 of 5 active`. */
export function describeActiveCount(
	{ running, total }: Readonly<ActiveCount>,
	i18n: I18n
): string {
	return t(i18n)`${running} of ${total} active`;
}
