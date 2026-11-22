import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

export function describeDetectedCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			0: 'No tunnel detected',
			one: '# tunnel detected',
			other: '# tunnels detected',
		})}`
	);
}

export function describeImportAction(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: 'Import # tunnel',
			other: 'Import # tunnels',
		})}`
	);
}

export function describeImportProgress(
	doneCount: number,
	totalCount: number,
	i18n: I18n
): string {
	return i18n._(msg`Import in progress: ${doneCount} of ${totalCount}`);
}

export function describeImportedCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: '# tunnel imported',
			other: '# tunnels imported',
		})}`
	);
}

export function describeConflictCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			0: 'No conflicts',
			one: '# conflict',
			other: '# conflicts',
		})}`
	);
}

/** What "Select all" or "Deselect all" ended with; `riskyCount` of the `count` need a closer look. */
export function describeSelectionCount(
	count: number,
	riskyCount: number,
	i18n: I18n
): string {
	const selected = i18n._(
		msg`${plural(count, {
			0: 'No tunnel selected',
			one: '# tunnel selected',
			other: '# tunnels selected',
		})}`
	);
	if (riskyCount === 0) return selected;
	return i18n._(msg`${selected}, including ${riskyCount} risky`);
}

export function describePreviewUpdated(
	selectedCount: number,
	isSelectionReset: boolean,
	i18n: I18n
): string {
	if (isSelectionReset) {
		return i18n._(
			msg`${plural(selectedCount, {
				0: 'Preview updated, selection reset, no tunnel selected',
				one: 'Preview updated, selection reset, # tunnel selected',
				other: 'Preview updated, selection reset, # tunnels selected',
			})}`
		);
	}
	return i18n._(
		msg`${plural(selectedCount, {
			0: 'Preview updated, no tunnel selected',
			one: 'Preview updated, # tunnel selected',
			other: 'Preview updated, # tunnels selected',
		})}`
	);
}
