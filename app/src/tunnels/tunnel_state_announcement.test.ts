import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { buildStateAnnouncement } from '#/tunnels/tunnel_state_announcement';

const active = (id: string, listenPort: number) =>
	buildTunnel({ id, listenPort, state: { kind: 'active' } });

describe('buildStateAnnouncement', () => {
	it('should announce the endpoint and the state of a single change', () => {
		expect(buildStateAnnouncement([active('a', 3333)])).toBe(
			'3333 vers nova:3333 : Actif'
		);
	});

	it('should include the reason of a failure', () => {
		const failed = buildTunnel({
			state: {
				kind: 'failed',
				reason: { kind: 'connect', message: 'timeout' },
			},
		});

		expect(buildStateAnnouncement([failed])).toBe(
			'3333 vers nova:3333 : Échec, Connexion SSH impossible : timeout'
		);
	});

	it('should announce a tunnel going to standby', () => {
		const standby = buildTunnel({ state: { kind: 'standby' } });

		expect(buildStateAnnouncement([standby])).toBe(
			'3333 vers nova:3333 : Veille'
		);
	});

	it('should include the owner of an external tunnel', () => {
		const external = buildTunnel({
			state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
		});

		expect(buildStateAnnouncement([external])).toBe(
			'3333 vers nova:3333 : Externe, fourni par ssh (pid 4242)'
		);
	});

	it('should count and detail up to three changes', () => {
		expect(buildStateAnnouncement([active('a', 3001), active('b', 3002)])).toBe(
			"2 tunnels ont changé d'état. 3001 vers nova:3001 : Actif. 3002 vers nova:3002 : Actif"
		);
	});

	it('should only count more than three changes', () => {
		const tunnels = ['a', 'b', 'c', 'd'].map((id, index) =>
			active(id, 3000 + index)
		);

		expect(buildStateAnnouncement(tunnels)).toBe("4 tunnels ont changé d'état");
	});

	it('should return an empty message without changes', () => {
		expect(buildStateAnnouncement([])).toBe('');
	});
});
