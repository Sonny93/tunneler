import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { ActiveCount } from '#/tunnels/active_count';

/** Visual form, such as `2/5 active`. */
export function formatActiveCount(
	{ running, total }: Readonly<ActiveCount>,
	i18n: I18n
): string {
	return i18n._(msg`${running}/${total} active`);
}
