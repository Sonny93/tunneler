import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** The polite status text for the number of options currently shown. */
export function describeOptionCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			0: 'No options shown',
			one: '# option shown',
			other: '# options shown',
		})}`
	);
}
