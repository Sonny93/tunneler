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

	it('should take the kind of the tunnel', () => {
		expect(toTunnelFormValues(buildTunnel({ kind: 'dynamic' })).kind).toBe(
			'dynamic'
		);
	});

	it('should join the tags with commas', () => {
		const values = toTunnelFormValues(buildTunnel({ tags: ['db', 'prod'] }));

		expect(values.tags).toBe('db, prod');
	});

	it('should show absent optional texts as blank and default the open options when the tunnel has none', () => {
		const values = toTunnelFormValues(buildTunnel());

		expect([values.targetHost, values.listenAddress]).toEqual(['', '']);
		expect([values.openScheme, values.openPath, values.openHost]).toEqual([
			'http',
			'',
			'',
		]);
	});

	it('should show the open options of the tunnel', () => {
		const values = toTunnelFormValues(
			buildTunnel({
				open: { scheme: 'https', path: '/admin', host: 'ip' },
			})
		);

		expect([values.openScheme, values.openPath, values.openHost]).toEqual([
			'https',
			'/admin',
			'ip',
		]);
	});

	it('should show the saved application, or automatic when none', () => {
		expect([
			toTunnelFormValues(buildTunnel({ open: { with: 'freerdp' } })).openWith,
			toTunnelFormValues(buildTunnel()).openWith,
		]).toEqual(['freerdp', '']);
	});

	it.each([
		[undefined, ''],
		[true, 'yes'],
		[false, 'no'],
	] as const)(
		'should show the standby override %s as "%s"',
		(standby, choice) => {
			expect(toTunnelFormValues(buildTunnel({ standby })).standby).toBe(choice);
		}
	);
});
