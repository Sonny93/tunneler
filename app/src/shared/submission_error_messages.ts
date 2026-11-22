import type { I18n } from '@lingui/core';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { isKnownImportField } from '#/daemon/describe_invalid_import_file';

const VALIDATION_FAILED_CODE = 'validation_failed';
const INVALID_IMPORT_FILE_CODE = 'invalid_import_file';

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

function readText(data: unknown, key: string): string | null {
	if (typeof data !== 'object' || data === null) return null;
	const value: unknown = Reflect.get(data, key);
	return typeof value === 'string' ? value : null;
}

function getImportFileEnglishTexts(
	error: Readonly<DaemonCommandError>
): string[] {
	const message = readText(error.data, 'message');
	const field = readText(error.data, 'field');
	return [
		...(message === null ? [] : [message]),
		...(field === null || isKnownImportField(field) ? [] : [field]),
	];
}

/**
 * The raw English the daemon wrote inside the messages of `getSubmissionErrorMessages`
 * (validation messages, an untranslated error message, a syntax error or an unknown
 * field of an import file), to mark with `lang="en"`.
 */
export function getEnglishTexts(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string[] {
	if (error.code === VALIDATION_FAILED_CODE && isMessageList(error.data)) {
		return error.data;
	}
	if (error.code === INVALID_IMPORT_FILE_CODE) {
		return getImportFileEnglishTexts(error);
	}
	return describeDaemonError(error, i18n) === error.message
		? [error.message]
		: [];
}
