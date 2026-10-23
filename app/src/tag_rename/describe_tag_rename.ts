import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** "Tag dev renamed to staging (3 tunnels)". */
export function describeTagRename(
	from: string,
	to: string,
	renamedCount: number,
	i18n: I18n
): string {
	return i18n._(
		msg`${plural(renamedCount, {
			one: `Tag ${from} renamed to ${to} (# tunnel)`,
			other: `Tag ${from} renamed to ${to} (# tunnels)`,
		})}`
	);
}
