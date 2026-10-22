import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeTunnelProblems } from '#/tunnels/describe_tunnel_problems';

describe('describeTunnelProblems', () => {
	it('should return null when the tunnel is healthy', () => {
		expect(
			describeTunnelProblems(
				{
					state: { kind: 'active' },
					health: { kind: 'reachable' },
				},
				i18n
			)
		).toBeNull();
	});

	it('should return the failure reason when the tunnel failed', () => {
		expect(
			describeTunnelProblems(
				{
					state: {
						kind: 'failed',
						reason: { kind: 'connect', message: 'boom' },
					},
					health: { kind: 'unknown' },
				},
				i18n
			)
		).toBe('SSH connection failed: boom');
	});

	it('should return the owner when the tunnel is external', () => {
		expect(
			describeTunnelProblems(
				{
					state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
					health: { kind: 'unknown' },
				},
				i18n
			)
		).toBe('provided by ssh (pid 4242)');
	});

	it('should return the health error when the target is unreachable', () => {
		expect(
			describeTunnelProblems(
				{
					state: { kind: 'active' },
					health: { kind: 'unreachable', since: 0, error: 'timeout' },
				},
				i18n
			)
		).toBe('timeout');
	});

	it('should join the failure reason and the health error when both exist', () => {
		expect(
			describeTunnelProblems(
				{
					state: {
						kind: 'failed',
						reason: { kind: 'connect', message: 'boom' },
					},
					health: { kind: 'unreachable', since: 0, error: 'timeout' },
				},
				i18n
			)
		).toBe('SSH connection failed: boom. timeout');
	});
});
