import { describe, expect, it } from 'vitest';

import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';

describe('getTunnelStateDetail', () => {
	it('should have no detail when a standby tunnel did not fail to wake up', () => {
		expect(getTunnelStateDetail({ kind: 'standby' })).toBeNull();
	});

	it('should give the last wake-up failure of a standby tunnel', () => {
		expect(
			getTunnelStateDetail({
				kind: 'standby',
				lastFailure: { kind: 'connect', message: 'refusé' },
			})
		).toBe('dernier réveil en échec : Connexion SSH impossible : refusé');
	});

	it('should name the process providing an external tunnel', () => {
		expect(
			getTunnelStateDetail({
				kind: 'external',
				owner: { pid: 4242, process: 'ssh' },
			})
		).toBe('fourni par ssh (pid 4242)');
	});

	it('should name the process holding the port of a conflict', () => {
		expect(
			getTunnelStateDetail({
				kind: 'conflict',
				owner: { pid: 4242, process: 'node.exe' },
			})
		).toBe('port occupé par node.exe (pid 4242)');
	});

	it('should say the process is unknown when a conflict has no owner', () => {
		expect(getTunnelStateDetail({ kind: 'conflict' })).toBe(
			'port occupé par un processus inconnu'
		);
	});

	it('should give the failure reason of a failed tunnel', () => {
		expect(
			getTunnelStateDetail({
				kind: 'failed',
				reason: { kind: 'connect', message: 'refusé' },
			})
		).toBe('Connexion SSH impossible : refusé');
	});

	it('should give no detail for a stopped tunnel', () => {
		expect(getTunnelStateDetail({ kind: 'stopped' })).toBeNull();
	});
});
