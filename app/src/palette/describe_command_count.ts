import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';

export function describeCommandCount(count: number, i18n: I18n): string {
	return t(i18n)`${plural(count, {
		0: 'No commands',
		one: '# command',
		other: '# commands',
	})}`;
}
