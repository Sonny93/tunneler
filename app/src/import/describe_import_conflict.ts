import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { ImportConflict } from '#/import/find_import_conflicts';

export function describeImportConflict(
	conflict: Readonly<ImportConflict>,
	i18n: I18n
): string {
	switch (conflict.kind) {
		case 'alreadyConfigured':
			return t(i18n)`Already configured`;
		case 'listenConflict': {
			const { tunnelLabel } = conflict;
			return t(i18n)`Port already used by ${tunnelLabel}`;
		}
		case 'duplicateInImport':
			return t(i18n)`Same port as a tunnel above`;
	}
}
