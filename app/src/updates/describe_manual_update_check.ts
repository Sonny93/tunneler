import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { ManualUpdateCheckOutcome } from '#/updates/run_manual_update_check';

type DescribedOutcome = Exclude<ManualUpdateCheckOutcome, { kind: 'skipped' }>;

/** The sentence that tells the user what a manual check found. */
export function describeManualUpdateCheck(
	outcome: Readonly<DescribedOutcome>,
	i18n: I18n
): string {
	switch (outcome.kind) {
		case 'upToDate':
			return i18n._(msg`You have the latest version`);
		case 'available': {
			const { version } = outcome;
			return i18n._(
				msg`Version ${version} is available: see the banner above the list`
			);
		}
		case 'failed':
			return i18n._(msg`Couldn't check for updates`);
	}
}
