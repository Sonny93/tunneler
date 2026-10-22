import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';

describe('listAwakeTunnels', () => {
	it('should keep only the awake tunnels', () => {
		const awake = buildTunnel({ id: 'a', isAwake: true });
		const asleep = buildTunnel({ id: 'b', isAwake: false });

		expect(listAwakeTunnels([awake, asleep])).toEqual([awake]);
	});

	it('should return nothing when no tunnel is awake', () => {
		expect(listAwakeTunnels([buildTunnel()])).toEqual([]);
	});
});
