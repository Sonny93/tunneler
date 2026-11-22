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

	it.each([
		[
			'devslot_not_found',
			{ host: 'vega', path: 'ds' },
			'devslot was not found on vega (looked for ds). The snippet to add to the configuration file is shown in the From devslot tab.',
		],
		[
			'devslot_failed',
			{ host: 'vega', status: 2, stderr: 'boom' },
			'devslot failed on vega (2): boom',
		],
		[
			'devslot_invalid_output',
			{ host: 'vega', reason: 'not json' },
			'devslot on vega answered something unusable: not json',
		],
		[
			'devslot_instance_not_found',
			{ host: 'vega', instance: 'web-shop-098a' },
			'vega does not list the devslot instance web-shop-098a',
		],
		[
			'remote_command_timeout',
			{ host: 'vega', seconds: 20 },
			'The command on vega did not finish within 20 s',
		],
	])('should describe the %s code from its data', (code, data, expected) => {
		const error = new DaemonCommandError(code, 'raw bridge text', data);

		expect(describeDaemonError(error, i18n)).toBe(expected);
	});

	it('should describe a connection failure with the host and the daemon message', () => {
		const error = new DaemonCommandError(
			'connection_failed',
			'Permission denied',
			{
				host: 'vega',
			}
		);

		expect(describeDaemonError(error, i18n)).toBe(
			'Could not connect to vega: Permission denied'
		);
	});

	it('should return the raw message when a connection failure has no host', () => {
		const error = new DaemonCommandError(
			'connection_failed',
			'Permission denied'
		);

		expect(describeDaemonError(error, i18n)).toBe('Permission denied');
	});

	it('should return the raw message when a devslot error has no usable data', () => {
		const error = new DaemonCommandError('devslot_not_found', 'raw text');

		expect(describeDaemonError(error, i18n)).toBe('raw text');
	});

	it('should return the raw message when the code is not mapped', () => {
		const error = new DaemonCommandError('not_found', 'unknown tunnel');

		expect(describeDaemonError(error, i18n)).toBe('unknown tunnel');
	});
});
