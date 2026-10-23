import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';

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
			return t(i18n)`The daemon is not reachable`;
		case 'internal':
			return t(i18n)`Internal error`;
		case 'update_failed':
			return t(i18n)`The update failed`;
		case 'update_unavailable':
			return t(i18n)`No update is available`;
		default:
			return error.message;
	}
}
