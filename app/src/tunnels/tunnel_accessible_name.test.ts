import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { getTunnelAccessibleName } from '#/tunnels/tunnel_accessible_name';

describe('getTunnelAccessibleName', () => {
	it('should combine name, endpoint and lowercase state', () => {
		expect(
			getTunnelAccessibleName(buildTunnel({ state: { kind: 'active' } }))
		).toBe('Tunnel Adonis, 3333 vers nova:3333, actif');
	});
});
