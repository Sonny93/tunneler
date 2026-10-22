import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

describe('describeDaemonError', () => {
	it.each([
		['disconnected', 'The daemon is not reachable'],
		['internal', 'Internal error'],
		['update_failed', 'The update failed'],
		['update_unavailable', 'No update is available'],
	])('should describe the %s code with a translated text', (code, expected) => {
		const error = new DaemonCommandError(code, 'raw bridge text');

		expect(describeDaemonError(error, i18n)).toBe(expected);
	});

	it('should return the raw message when the code is not mapped', () => {
		const error = new DaemonCommandError('not_found', 'unknown tunnel');

		expect(describeDaemonError(error, i18n)).toBe('unknown tunnel');
	});
});
