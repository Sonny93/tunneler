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

describe('getTunnelEndpointLabel for a remote tunnel', () => {
	const remote = buildTunnel({
		kind: 'remote',
		listenPort: 8080,
		targetPort: 3000,
	});

	it('should name the server port and the target seen from this machine', () => {
		expect(getTunnelEndpointLabel(remote)).toBe('nova:8080 → 127.0.0.1:3000');
	});

	it('should default the target port to the listen port', () => {
		expect(
			getTunnelEndpointLabel({
				...remote,
				targetPort: undefined,
				targetHost: 'app',
			})
		).toBe('nova:8080 → app:8080');
	});

	it('should say it in words', () => {
		expect(getTunnelAccessibleEndpointLabel(remote)).toBe(
			'port 8080 du serveur nova vers 127.0.0.1:3000'
		);
	});
});

describe('getTunnelEndpointLabel for a dynamic tunnel', () => {
	const dynamic = buildTunnel({ kind: 'dynamic', listenPort: 1080 });

	it('should mention SOCKS and the ssh host', () => {
		expect(getTunnelEndpointLabel(dynamic)).toBe('1080 (SOCKS via nova)');
	});

	it('should say it in words', () => {
		expect(getTunnelAccessibleEndpointLabel(dynamic)).toBe(
			'proxy SOCKS sur le port 1080 via nova'
		);
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
