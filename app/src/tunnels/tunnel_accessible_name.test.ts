import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { getTunnelAccessibleName } from '#/tunnels/tunnel_accessible_name';

describe('getTunnelAccessibleName', () => {
	it('should combine endpoint and lowercase state', () => {
		expect(
			getTunnelAccessibleName(buildTunnel({ state: { kind: 'active' } }))
		).toBe('Tunnel 3333 vers nova:3333, actif');
	});

	it('should name the external state', () => {
		expect(
			getTunnelAccessibleName(
				buildTunnel({
					state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
				})
			)
		).toBe('Tunnel 3333 vers nova:3333, externe');
	});
});

describe('getTunnelAccessibleName for a temporary tunnel', () => {
	it('should say temporaire before the state', () => {
		expect(
			getTunnelAccessibleName(
				buildTunnel({ isTemporary: true, state: { kind: 'active' } })
			)
		).toBe('Tunnel 3333 vers nova:3333, temporaire, actif');
	});
});
