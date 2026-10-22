import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { buildStateAnnouncement } from '#/tunnels/tunnel_state_announcement';

const active = (id: string, listenPort: number) =>
	buildTunnel({ id, listenPort, state: { kind: 'active' } });

describe('buildStateAnnouncement', () => {
	it('should announce the endpoint and the state of a single change', () => {
		expect(buildStateAnnouncement([active('a', 3333)], i18n)).toBe(
			'3333 to nova:3333: Active'
		);
	});

	it('should include the reason of a failure', () => {
		const failed = buildTunnel({
			state: {
				kind: 'failed',
				reason: { kind: 'connect', message: 'timeout' },
			},
		});

		expect(buildStateAnnouncement([failed], i18n)).toBe(
			'3333 to nova:3333: Failed, SSH connection failed: timeout'
		);
	});

	it('should announce a tunnel going to standby', () => {
		const standby = buildTunnel({ state: { kind: 'standby' } });

		expect(buildStateAnnouncement([standby], i18n)).toBe(
			'3333 to nova:3333: Standby'
		);
	});

	it('should include the owner of an external tunnel', () => {
		const external = buildTunnel({
			state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
		});

		expect(buildStateAnnouncement([external], i18n)).toBe(
			'3333 to nova:3333: External, provided by ssh (pid 4242)'
		);
	});

	it('should count and detail up to three changes', () => {
		expect(
			buildStateAnnouncement([active('a', 3001), active('b', 3002)], i18n)
		).toBe(
			'2 tunnels changed state. 3001 to nova:3001: Active. 3002 to nova:3002: Active'
		);
	});

	it('should only count more than three changes', () => {
		const tunnels = ['a', 'b', 'c', 'd'].map((id, index) =>
			active(id, 3000 + index)
		);

		expect(buildStateAnnouncement(tunnels, i18n)).toBe(
			'4 tunnels changed state'
		);
	});

	it('should return an empty message without changes', () => {
		expect(buildStateAnnouncement([], i18n)).toBe('');
	});
});
