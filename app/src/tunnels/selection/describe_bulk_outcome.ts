import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';

import type { ToastTone } from '#/shared/toasts/toast_store';
import type {
	BulkFailure,
	BulkOutcome,
} from '#/tunnels/selection/run_bulk_operation';

/** What a bulk action did to the tunnels, to word its report. */
export type BulkVerb =
	| 'start'
	| 'stop'
	| 'restart'
	| 'standby'
	| 'remove'
	| 'update';

type BulkReport = { tone: ToastTone; message: string };

type PartialOutcome = {
	succeededCount: number;
	total: number;
	failureList: string;
};

function describeFailures(failures: readonly BulkFailure[]): string {
	return failures
		.map(({ label, message }) => `${label} (${message})`)
		.join(', ');
}

function describeSuccess(verb: BulkVerb, total: number, i18n: I18n): string {
	switch (verb) {
		case 'start':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel started', other: '# tunnels started' })}`;
		case 'stop':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel stopped', other: '# tunnels stopped' })}`;
		case 'restart':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel restarted', other: '# tunnels restarted' })}`;
		case 'standby':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel sent to standby', other: '# tunnels sent to standby' })}`;
		case 'remove':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel deleted', other: '# tunnels deleted' })}`;
		case 'update':
			return t(
				i18n
			)`${plural(total, { one: '# tunnel updated', other: '# tunnels updated' })}`;
	}
}

function describePartialFailure(
	verb: BulkVerb,
	{ succeededCount, total, failureList }: Readonly<PartialOutcome>,
	i18n: I18n
): string {
	switch (verb) {
		case 'start':
			return t(
				i18n
			)`${succeededCount} of ${total} started; failed: ${failureList}`;
		case 'stop':
			return t(
				i18n
			)`${succeededCount} of ${total} stopped; failed: ${failureList}`;
		case 'restart':
			return t(
				i18n
			)`${succeededCount} of ${total} restarted; failed: ${failureList}`;
		case 'standby':
			return t(
				i18n
			)`${succeededCount} of ${total} sent to standby; failed: ${failureList}`;
		case 'remove':
			return t(
				i18n
			)`${succeededCount} of ${total} deleted; failed: ${failureList}`;
		case 'update':
			return t(
				i18n
			)`${succeededCount} of ${total} updated; failed: ${failureList}`;
	}
}

/** `3 tunnels started`, or `2 of 3 started; failed: 3000 → nova:3000 (message)` in the error tone. */
export function describeBulkOutcome(
	{ total, succeededIds, failures }: Readonly<BulkOutcome>,
	verb: BulkVerb,
	i18n: I18n
): BulkReport {
	if (failures.length === 0) {
		return { tone: 'success', message: describeSuccess(verb, total, i18n) };
	}
	return {
		tone: 'error',
		message: describePartialFailure(
			verb,
			{
				succeededCount: succeededIds.length,
				total,
				failureList: describeFailures(failures),
			},
			i18n
		),
	};
}
