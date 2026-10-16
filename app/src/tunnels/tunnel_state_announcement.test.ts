import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { buildStateAnnouncement } from '#/tunnels/tunnel_state_announcement';

const active = (id: string, name: string) =>
	buildTunnel({ id, name, state: { kind: 'active' } });

describe('buildStateAnnouncement', () => {
	it('should announce the name and the state of a single change', () => {
		expect(buildStateAnnouncement([active('a', 'Adonis')])).toBe(
			'Adonis : Actif'
		);
	});

	it('should include the reason of a failure', () => {
		const failed = buildTunnel({
			name: 'Adonis',
			state: {
				kind: 'failed',
				reason: { kind: 'connect', message: 'timeout' },
			},
		});

		expect(buildStateAnnouncement([failed])).toBe(
			'Adonis : Échec, Connexion SSH impossible : timeout'
		);
	});

	it('should include the owner of an external tunnel', () => {
		const external = buildTunnel({
			name: 'Adonis',
			state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
		});

		expect(buildStateAnnouncement([external])).toBe(
			'Adonis : Externe, fourni par ssh (pid 4242)'
		);
	});

	it('should count and detail up to three changes', () => {
		expect(buildStateAnnouncement([active('a', 'A'), active('b', 'B')])).toBe(
			"2 tunnels ont changé d'état. A : Actif. B : Actif"
		);
	});

	it('should only count more than three changes', () => {
		const tunnels = ['a', 'b', 'c', 'd'].map((id) => active(id, id));

		expect(buildStateAnnouncement(tunnels)).toBe("4 tunnels ont changé d'état");
	});

	it('should return an empty message without changes', () => {
		expect(buildStateAnnouncement([])).toBe('');
	});
});
