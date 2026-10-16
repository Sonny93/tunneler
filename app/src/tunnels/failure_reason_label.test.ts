import { describe, expect, it } from 'vitest';

import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

describe('getFailureReasonLabel', () => {
	it('should describe a port already in use', () => {
		expect(
			getFailureReasonLabel({ kind: 'portInUse', address: '127.0.0.1:3333' })
		).toBe('Port déjà utilisé (127.0.0.1:3333)');
	});

	it('should describe a listen failure', () => {
		expect(
			getFailureReasonLabel({
				kind: 'listenFailed',
				address: '0.0.0.0:80',
				message: 'permission denied',
			})
		).toBe('Écoute impossible sur 0.0.0.0:80 : permission denied');
	});

	it('should describe a connection failure', () => {
		expect(getFailureReasonLabel({ kind: 'connect', message: 'timeout' })).toBe(
			'Connexion SSH impossible : timeout'
		);
	});

	it('should describe an unsupported kind', () => {
		expect(
			getFailureReasonLabel({ kind: 'unsupportedKind', tunnelKind: 'dynamic' })
		).toBe('Type de tunnel non pris en charge (dynamic)');
	});
});
