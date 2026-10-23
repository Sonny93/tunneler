import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { ImportConflict } from '#/import/find_import_conflicts';

export function describeImportConflict(
	conflict: Readonly<ImportConflict>,
	i18n: I18n
): string {
	switch (conflict.kind) {
		case 'alreadyConfigured':
			return i18n._(msg`Already configured`);
		case 'listenConflict': {
			const { tunnelLabel } = conflict;
			return i18n._(msg`Port already used by ${tunnelLabel}`);
		}
		case 'duplicateInImport':
			return i18n._(msg`Same port as a tunnel above`);
	}
}
