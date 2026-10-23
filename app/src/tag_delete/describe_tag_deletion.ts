import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** "The tag will be removed from 3 tunnels. The tunnels are kept.", plus the launch tags when the tag is one of them. */
export function describeTagDeletionWarning(
	tunnelCount: number,
	isLaunchTag: boolean,
	i18n: I18n
): string {
	const warning = i18n._(
		msg`${plural(tunnelCount, {
			one: 'The tag will be removed from # tunnel. The tunnels are kept.',
			other: 'The tag will be removed from # tunnels. The tunnels are kept.',
		})}`
	);
	if (!isLaunchTag) return warning;
	const launchNote = i18n._(
		msg`It will also be removed from the "Tags to start on launch".`
	);
	return `${warning} ${launchNote}`;
}

/** "Tag dev removed from 3 tunnels". */
export function describeTagDeletion(
	tag: string,
	removedCount: number,
	i18n: I18n
): string {
	return i18n._(
		msg`${plural(removedCount, {
			one: `Tag ${tag} removed from # tunnel`,
			other: `Tag ${tag} removed from # tunnels`,
		})}`
	);
}
