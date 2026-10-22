import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

describe('getFailureReasonLabel', () => {
	it('should describe a port already in use', () => {
		expect(
			getFailureReasonLabel(
				{ kind: 'portInUse', address: '127.0.0.1:3333' },
				i18n
			)
		).toBe('Port already in use (127.0.0.1:3333)');
	});

	it('should describe a listen failure', () => {
		expect(
			getFailureReasonLabel(
				{
					kind: 'listenFailed',
					address: '0.0.0.0:80',
					message: 'permission denied',
				},
				i18n
			)
		).toBe('Cannot listen on 0.0.0.0:80: permission denied');
	});

	it('should describe a connection failure', () => {
		expect(
			getFailureReasonLabel({ kind: 'connect', message: 'timeout' }, i18n)
		).toBe('SSH connection failed: timeout');
	});

	it('should describe an unresponsive agent in seconds', () => {
		expect(
			getFailureReasonLabel(
				{ kind: 'agentUnresponsive', timeoutMs: 30_000 },
				i18n
			)
		).toBe(
			'The SSH agent is not responding (no answer in 30 s): is it waiting for a confirmation?'
		);
	});

	it('should describe a refused remote forward', () => {
		expect(
			getFailureReasonLabel(
				{
					kind: 'remoteForwardRefused',
					address: '0.0.0.0:8080',
				},
				i18n
			)
		).toBe('The server refuses to listen on 0.0.0.0:8080');
	});
});
