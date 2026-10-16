import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';

describe('getSubmissionErrorMessages', () => {
	it('should return the daemon messages when validation failed', () => {
		const error = new DaemonCommandError('validation_failed', 'invalide', [
			'port déjà utilisé',
			'nom en double',
		]);

		expect(getSubmissionErrorMessages(error)).toEqual([
			'port déjà utilisé',
			'nom en double',
		]);
	});

	it('should return the error message when validation data is not a list of strings', () => {
		const error = new DaemonCommandError('validation_failed', 'invalide', 3);

		expect(getSubmissionErrorMessages(error)).toEqual(['invalide']);
	});

	it('should return the error message for any other code', () => {
		const error = new DaemonCommandError('io_error', 'disque plein');

		expect(getSubmissionErrorMessages(error)).toEqual(['disque plein']);
	});
});
