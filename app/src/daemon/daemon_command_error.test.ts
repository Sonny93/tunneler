import { describe, expect, it } from 'vitest';

import {
	DaemonCommandError,
	toDaemonCommandError,
} from '#/daemon/daemon_command_error';

describe('toDaemonCommandError', () => {
	it('should keep the code, message and data of a native rejection', () => {
		const error = toDaemonCommandError({
			code: 'validation_failed',
			message: 'invalid',
			data: ['name is empty'],
		});

		expect(error).toMatchObject({
			code: 'validation_failed',
			message: 'invalid',
			data: ['name is empty'],
		});
	});

	it('should produce a DaemonCommandError instance', () => {
		expect(
			toDaemonCommandError({ code: 'not_found', message: 'x' })
		).toBeInstanceOf(DaemonCommandError);
	});

	it('should return the same error when it is already converted', () => {
		const error = new DaemonCommandError('disconnected', 'gone');

		expect(toDaemonCommandError(error)).toBe(error);
	});

	it('should use the unexpected code for an unknown object', () => {
		expect(toDaemonCommandError({ foo: 'bar' }).code).toBe('unexpected');
	});

	it('should use the message of a native Error with the unexpected code', () => {
		expect(toDaemonCommandError(new Error('boom'))).toMatchObject({
			code: 'unexpected',
			message: 'boom',
		});
	});

	it('should use a string rejection as the message', () => {
		expect(toDaemonCommandError('oops')).toMatchObject({
			code: 'unexpected',
			message: 'oops',
		});
	});

	it('should treat a rejection with a non-string code as unexpected', () => {
		expect(toDaemonCommandError({ code: 12, message: 'x' }).code).toBe(
			'unexpected'
		);
	});
});
