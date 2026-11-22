import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';

function readText(data: unknown, key: string): string | null {
	if (typeof data !== 'object' || data === null) return null;
	const value: unknown = Reflect.get(data, key);
	return typeof value === 'string' || typeof value === 'number'
		? String(value)
		: null;
}

function describeDevslotFailure(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const host = readText(error.data, 'host');
	const status = readText(error.data, 'status');
	const stderr = readText(error.data, 'stderr');
	if (host === null) return error.message;
	const statusText = status === null ? '' : ` (${status})`;
	if (stderr === null || stderr === '') {
		return i18n._(msg`devslot failed on ${host}${statusText}`);
	}
	return i18n._(msg`devslot failed on ${host}${statusText}: ${stderr}`);
}

function describeDevslotNotFound(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const host = readText(error.data, 'host');
	const path = readText(error.data, 'path');
	if (host === null || path === null) return error.message;
	return i18n._(
		msg`devslot was not found on ${host} (looked for ${path}). The snippet to add to the configuration file is shown in the From devslot tab.`
	);
}

function describeDevslotInvalidOutput(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const host = readText(error.data, 'host');
	const reason = readText(error.data, 'reason');
	if (host === null || reason === null) return error.message;
	return i18n._(msg`devslot on ${host} answered something unusable: ${reason}`);
}

function describeConnectionFailed(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const host = readText(error.data, 'host');
	if (host === null) return error.message;
	const { message } = error;
	return i18n._(msg`Could not connect to ${host}: ${message}`);
}

function describeCommandTimeout(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const host = readText(error.data, 'host');
	const seconds = readText(error.data, 'seconds');
	if (host === null || seconds === null) return error.message;
	return i18n._(msg`The command on ${host} did not finish within ${seconds} s`);
}

/**
 * The text to show for a bridge or daemon failure: a translation chosen by the
 * error code when the code is known, the raw message otherwise. Failures raised by
 * the bridge itself (not by the daemon) carry English messages, hence the mapping.
 */
export function describeDaemonError(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	switch (error.code) {
		case 'disconnected':
			return i18n._(msg`The daemon is not reachable`);
		case 'internal':
			return i18n._(msg`Internal error`);
		case 'update_failed':
			return i18n._(msg`The update failed`);
		case 'update_unavailable':
			return i18n._(msg`No update is available`);
		case 'devslot_not_found':
			return describeDevslotNotFound(error, i18n);
		case 'devslot_failed':
			return describeDevslotFailure(error, i18n);
		case 'devslot_invalid_output':
			return describeDevslotInvalidOutput(error, i18n);
		case 'connection_failed':
			return describeConnectionFailed(error, i18n);
		case 'remote_command_timeout':
			return describeCommandTimeout(error, i18n);
		default:
			return error.message;
	}
}
