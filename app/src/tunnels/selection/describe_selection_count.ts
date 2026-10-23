import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** `1 tunnel`, `3 tunnels`. */
export function formatTunnelCount(count: number, i18n: I18n): string {
	return i18n._(msg`${plural(count, { one: '# tunnel', other: '# tunnels' })}`);
}

/** `No tunnels selected`, `1 tunnel selected`, `3 tunnels selected`. */
export function describeSelectionCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			0: 'No tunnels selected',
			one: '# tunnel selected',
			other: '# tunnels selected',
		})}`
	);
}
