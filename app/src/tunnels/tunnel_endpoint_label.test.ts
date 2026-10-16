import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

describe('getTunnelEndpointLabel', () => {
	it('should default the target port to the listen port', () => {
		expect(getTunnelEndpointLabel(buildTunnel())).toBe('3333 → nova:3333');
	});

	it('should use the target port when it is set', () => {
		expect(getTunnelEndpointLabel(buildTunnel({ targetPort: 80 }))).toBe(
			'3333 → nova:80'
		);
	});

	it.each(['127.0.0.1', 'localhost', '::1'])(
		'should treat %s as the ssh host itself',
		(targetHost) => {
			expect(getTunnelEndpointLabel(buildTunnel({ targetHost }))).toBe(
				'3333 → nova:3333'
			);
		}
	);

	it('should name the target and the ssh host for a remote target', () => {
		expect(
			getTunnelEndpointLabel(
				buildTunnel({ targetHost: 'db.internal', targetPort: 5432 })
			)
		).toBe('3333 → db.internal:5432 via nova');
	});
});

describe('getTunnelAccessibleEndpointLabel', () => {
	it('should use vers instead of the arrow', () => {
		expect(getTunnelAccessibleEndpointLabel(buildTunnel())).toBe(
			'3333 vers nova:3333'
		);
	});

	it('should keep the via part for a remote target', () => {
		expect(
			getTunnelAccessibleEndpointLabel(
				buildTunnel({ targetHost: 'db', targetPort: 1 })
			)
		).toBe('3333 vers db:1 via nova');
	});
});
