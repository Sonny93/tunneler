import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';

export function describeDetectedCount(count: number, i18n: I18n): string {
	return t(i18n)`${plural(count, {
		0: 'No tunnel detected',
		one: '# tunnel detected',
		other: '# tunnels detected',
	})}`;
}

export function describeImportAction(count: number, i18n: I18n): string {
	return t(i18n)`${plural(count, {
		one: 'Import # tunnel',
		other: 'Import # tunnels',
	})}`;
}

export function describeImportProgress(
	doneCount: number,
	totalCount: number,
	i18n: I18n
): string {
	return t(i18n)`Import in progress: ${doneCount} of ${totalCount}`;
}

export function describeImportedCount(count: number, i18n: I18n): string {
	return t(i18n)`${plural(count, {
		one: '# tunnel imported',
		other: '# tunnels imported',
	})}`;
}
