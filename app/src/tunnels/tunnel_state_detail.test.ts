import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';

describe('getTunnelStateDetail', () => {
	it('should have no detail when a standby tunnel did not fail to wake up', () => {
		expect(getTunnelStateDetail({ kind: 'standby' }, i18n)).toBeNull();
	});

	it('should give the last wake-up failure of a standby tunnel', () => {
		expect(
			getTunnelStateDetail(
				{
					kind: 'standby',
					lastFailure: { kind: 'connect', message: 'refused' },
				},
				i18n
			)
		).toBe('last wake-up failed: SSH connection failed: refused');
	});

	it('should name the process providing an external tunnel', () => {
		expect(
			getTunnelStateDetail(
				{
					kind: 'external',
					owner: { pid: 4242, process: 'ssh' },
				},
				i18n
			)
		).toBe('provided by ssh (pid 4242)');
	});

	it('should name the process holding the port of a conflict', () => {
		expect(
			getTunnelStateDetail(
				{
					kind: 'conflict',
					owner: { pid: 4242, process: 'node.exe' },
				},
				i18n
			)
		).toBe('port in use by node.exe (pid 4242)');
	});

	it('should say the process is unknown when a conflict has no owner', () => {
		expect(getTunnelStateDetail({ kind: 'conflict' }, i18n)).toBe(
			'port in use by an unknown process'
		);
	});

	it('should give the failure reason of a failed tunnel', () => {
		expect(
			getTunnelStateDetail(
				{
					kind: 'failed',
					reason: { kind: 'connect', message: 'refused' },
				},
				i18n
			)
		).toBe('SSH connection failed: refused');
	});

	it('should give no detail for a stopped tunnel', () => {
		expect(getTunnelStateDetail({ kind: 'stopped' }, i18n)).toBeNull();
	});
});
