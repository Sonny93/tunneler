import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** `1 open connection will be cut.`, `2 open connections will be cut.` */
export function describeCutConnections(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: '# open connection will be cut.',
			other: '# open connections will be cut.',
		})}`
	);
}
