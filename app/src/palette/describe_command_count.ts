import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

export function describeCommandCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			0: 'No commands',
			one: '# command',
			other: '# commands',
		})}`
	);
}
