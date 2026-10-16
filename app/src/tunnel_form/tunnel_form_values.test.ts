import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import {
	EMPTY_TUNNEL_FORM_VALUES,
	toTunnelFormValues,
} from '#/tunnel_form/tunnel_form_values';

describe('toTunnelFormValues', () => {
	it('should return the empty linked form when creating', () => {
		expect(toTunnelFormValues(null)).toEqual(EMPTY_TUNNEL_FORM_VALUES);
	});

	it('should link the target port when the tunnel has none', () => {
		const values = toTunnelFormValues(buildTunnel({ listenPort: 3333 }));

		expect([values.isTargetPortLinked, values.targetPort]).toEqual([
			true,
			'3333',
		]);
	});

	it('should unlink the target port when the tunnel has one', () => {
		const values = toTunnelFormValues(
			buildTunnel({ listenPort: 3333, targetPort: 5432 })
		);

		expect([values.isTargetPortLinked, values.targetPort]).toEqual([
			false,
			'5432',
		]);
	});

	it('should join the tags with commas', () => {
		const values = toTunnelFormValues(buildTunnel({ tags: ['db', 'prod'] }));

		expect(values.tags).toBe('db, prod');
	});

	it('should show absent optional texts as blank', () => {
		const values = toTunnelFormValues(buildTunnel());

		expect([values.targetHost, values.listenAddress]).toEqual(['', '']);
	});
});
