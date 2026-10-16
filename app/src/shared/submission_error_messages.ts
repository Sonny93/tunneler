import type { DaemonCommandError } from '#/daemon/daemon_command_error';

const VALIDATION_FAILED_CODE = 'validation_failed';

function isMessageList(value: unknown): value is string[] {
	return (
		Array.isArray(value) &&
		value.length > 0 &&
		value.every((entry) => typeof entry === 'string')
	);
}

/** The daemon's validation messages when it sent some, otherwise the error message alone. */
export function getSubmissionErrorMessages(
	error: Readonly<DaemonCommandError>
): string[] {
	if (error.code === VALIDATION_FAILED_CODE && isMessageList(error.data)) {
		return error.data;
	}
	return [error.message];
}
