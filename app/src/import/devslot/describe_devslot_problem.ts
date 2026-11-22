import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import { describeImportConflict } from '#/import/describe_import_conflict';
import type { DevslotPortProblem } from '#/import/devslot/find_devslot_problems';

export function describeDevslotProblem(
	problem: Readonly<DevslotPortProblem>,
	i18n: I18n
): string {
	if (problem.kind === 'invalidLocalPort') {
		return i18n._(msg`Enter a port between 1 and 65535.`);
	}
	return describeImportConflict(problem, i18n);
}
