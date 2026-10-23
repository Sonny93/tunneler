import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { ActiveCount } from '#/tunnels/active_count';

/** Visual form, such as `2/5 active`. */
export function formatActiveCount(
	{ running, total }: Readonly<ActiveCount>,
	i18n: I18n
): string {
	return t(i18n)`${running}/${total} active`;
}
