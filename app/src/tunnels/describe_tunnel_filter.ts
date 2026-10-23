import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { TunnelFilter } from '#/state/ui_store';

export function describeTunnelFilter(
	filter: Readonly<TunnelFilter>,
	i18n: I18n
): string {
	switch (filter.kind) {
		case 'all':
			return i18n._(msg`All tunnels`);
		case 'tag': {
			const { tag } = filter;
			return i18n._(msg`Tag ${tag}`);
		}
		case 'host': {
			const { host } = filter;
			return i18n._(msg`Host ${host}`);
		}
	}
}
