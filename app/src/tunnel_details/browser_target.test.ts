import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { getBrowserTarget } from '#/tunnel_details/browser_target';

describe('getBrowserTarget', () => {
	it('should be null for a remote forward', () => {
		expect(
			getBrowserTarget(buildTunnel({ kind: 'remote' }), '127.0.0.1')
		).toBeNull();
	});

	it('should be null for a dynamic forward', () => {
		expect(
			getBrowserTarget(buildTunnel({ kind: 'dynamic' }), '127.0.0.1')
		).toBeNull();
	});

	it('should use the default bind address when the tunnel sets none', () => {
		expect(getBrowserTarget(buildTunnel(), '127.0.0.1')).toEqual({
			host: '127.0.0.1',
			port: 3333,
		});
	});

	it('should prefer the listen address of the tunnel', () => {
		expect(
			getBrowserTarget(buildTunnel({ listenAddress: '127.0.0.2' }), '0.0.0.0')
		).toEqual({ host: '127.0.0.2', port: 3333 });
	});

	it.each(['0.0.0.0', '::', '*', ''])(
		'should reach the wildcard address "%s" through localhost',
		(address) => {
			expect(
				getBrowserTarget(buildTunnel({ listenAddress: address }), '127.0.0.1')
			).toEqual({ host: 'localhost', port: 3333 });
		}
	);

	it('should reach localhost when no default bind address is known', () => {
		expect(getBrowserTarget(buildTunnel(), undefined)).toEqual({
			host: 'localhost',
			port: 3333,
		});
	});
});
