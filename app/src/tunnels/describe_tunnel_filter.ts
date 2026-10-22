import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { TunnelFilter } from '#/state/ui_store';

export function describeTunnelFilter(
	filter: Readonly<TunnelFilter>,
	i18n: I18n
): string {
	switch (filter.kind) {
		case 'all':
			return t(i18n)`All tunnels`;
		case 'tag': {
			const { tag } = filter;
			return t(i18n)`Tag ${tag}`;
		}
		case 'host': {
			const { host } = filter;
			return t(i18n)`Host ${host}`;
		}
	}
}
