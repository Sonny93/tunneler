import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';

describe('getSubmissionErrorMessages', () => {
	it('should return the daemon messages when validation failed', () => {
		const error = new DaemonCommandError('validation_failed', 'invalid', [
			'port already in use',
			'duplicate name',
		]);

		expect(getSubmissionErrorMessages(error, i18n)).toEqual([
			'port already in use',
			'duplicate name',
		]);
	});

	it('should return the error message when validation data is not a list of strings', () => {
		const error = new DaemonCommandError('validation_failed', 'invalid', 3);

		expect(getSubmissionErrorMessages(error, i18n)).toEqual(['invalid']);
	});

	it('should return the error message for any other code', () => {
		const error = new DaemonCommandError('io_error', 'disk full');

		expect(getSubmissionErrorMessages(error, i18n)).toEqual(['disk full']);
	});

	it('should describe the error by its code when the code is known', () => {
		const error = new DaemonCommandError('disconnected', 'raw bridge text');

		expect(getSubmissionErrorMessages(error, i18n)).toEqual([
			'The daemon is not reachable',
		]);
	});
});
