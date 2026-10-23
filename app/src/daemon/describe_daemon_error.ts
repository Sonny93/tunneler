import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

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
			return i18n._(msg`The daemon is not reachable`);
		case 'internal':
			return i18n._(msg`Internal error`);
		case 'update_failed':
			return i18n._(msg`The update failed`);
		case 'update_unavailable':
			return i18n._(msg`No update is available`);
		default:
			return error.message;
	}
}
