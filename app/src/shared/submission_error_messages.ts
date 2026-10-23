import type { I18n } from '@lingui/core';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

const VALIDATION_FAILED_CODE = 'validation_failed';

function isMessageList(value: unknown): value is string[] {
	return (
		Array.isArray(value) &&
		value.length > 0 &&
		value.every((entry) => typeof entry === 'string')
	);
}

/** The daemon's validation messages when it sent some, otherwise the description of the error alone. */
export function getSubmissionErrorMessages(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string[] {
	if (error.code === VALIDATION_FAILED_CODE && isMessageList(error.data)) {
		return error.data;
	}
	return [describeDaemonError(error, i18n)];
}
