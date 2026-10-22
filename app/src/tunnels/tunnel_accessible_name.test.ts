import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { getTunnelAccessibleName } from '#/tunnels/tunnel_accessible_name';

describe('getTunnelAccessibleName', () => {
	it('should combine endpoint and lowercase state', () => {
		expect(
			getTunnelAccessibleName(buildTunnel({ state: { kind: 'active' } }), i18n)
		).toBe('Tunnel 3333 to nova:3333, active');
	});

	it('should name the standby state', () => {
		expect(
			getTunnelAccessibleName(buildTunnel({ state: { kind: 'standby' } }), i18n)
		).toBe('Tunnel 3333 to nova:3333, standby');
	});

	it('should name the external state', () => {
		expect(
			getTunnelAccessibleName(
				buildTunnel({
					state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
				}),
				i18n
			)
		).toBe('Tunnel 3333 to nova:3333, external');
	});
});

describe('getTunnelAccessibleName for a temporary tunnel', () => {
	it('should say temporary before the state', () => {
		expect(
			getTunnelAccessibleName(
				buildTunnel({ isTemporary: true, state: { kind: 'active' } }),
				i18n
			)
		).toBe('Tunnel 3333 to nova:3333, temporary, active');
	});
});
