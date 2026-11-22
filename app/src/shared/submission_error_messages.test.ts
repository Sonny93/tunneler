import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import {
	getEnglishTexts,
	getSubmissionErrorMessages,
} from '#/shared/submission_error_messages';

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

describe('getEnglishTexts', () => {
	it('should return the validation messages of the daemon', () => {
		const error = new DaemonCommandError('validation_failed', 'invalid', [
			'port in use',
		]);

		expect(getEnglishTexts(error, i18n)).toEqual(['port in use']);
	});

	it('should return the parser message of an import file syntax error', () => {
		const error = new DaemonCommandError('invalid_import_file', 'raw', {
			reason: 'syntax',
			message: 'bad key',
		});

		expect(getEnglishTexts(error, i18n)).toEqual(['bad key']);
	});

	it('should return the field of an import file that has no translated name', () => {
		const error = new DaemonCommandError('invalid_import_file', 'raw', {
			reason: 'unsafeCharacters',
			entry: 0,
			field: 'open.future',
		});

		expect(getEnglishTexts(error, i18n)).toEqual(['open.future']);
	});

	it('should return the raw message of an error the UI does not translate', () => {
		expect(
			getEnglishTexts(new DaemonCommandError('io_error', 'disk full'), i18n)
		).toEqual(['disk full']);
	});

	it('should return nothing for an error the UI translates', () => {
		expect(
			getEnglishTexts(new DaemonCommandError('disconnected', 'lost'), i18n)
		).toEqual([]);
	});
});
