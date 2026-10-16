import { describe, expect, it } from 'vitest';

import { describeTunnelProblems } from '#/tunnels/describe_tunnel_problems';

describe('describeTunnelProblems', () => {
	it('should return null when the tunnel is healthy', () => {
		expect(
			describeTunnelProblems({
				state: { kind: 'active' },
				health: { kind: 'reachable' },
			})
		).toBeNull();
	});

	it('should return the failure reason when the tunnel failed', () => {
		expect(
			describeTunnelProblems({
				state: { kind: 'failed', reason: { kind: 'connect', message: 'boom' } },
				health: { kind: 'unknown' },
			})
		).toBe('Connexion SSH impossible : boom');
	});

	it('should return the health error when the target is unreachable', () => {
		expect(
			describeTunnelProblems({
				state: { kind: 'active' },
				health: { kind: 'unreachable', since: 0, error: 'timeout' },
			})
		).toBe('timeout');
	});

	it('should join the failure reason and the health error when both exist', () => {
		expect(
			describeTunnelProblems({
				state: { kind: 'failed', reason: { kind: 'connect', message: 'boom' } },
				health: { kind: 'unreachable', since: 0, error: 'timeout' },
			})
		).toBe('Connexion SSH impossible : boom. timeout');
	});
});
