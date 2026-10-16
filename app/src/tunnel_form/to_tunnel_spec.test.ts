import { describe, expect, it } from 'vitest';

import { toTunnelSpec } from '#/tunnel_form/to_tunnel_spec';
import { EMPTY_TUNNEL_FORM_VALUES } from '#/tunnel_form/tunnel_form_values';

const valid = {
	...EMPTY_TUNNEL_FORM_VALUES,
	name: ' Adonis ',
	host: 'nova',
	listenPort: '3333',
	targetPort: '3333',
};

describe('toTunnelSpec', () => {
	it('should omit the target port while linked', () => {
		expect(toTunnelSpec(valid, 'local').targetPort).toBeUndefined();
	});

	it('should send the target port once unlinked', () => {
		const spec = toTunnelSpec(
			{ ...valid, targetPort: '5432', isTargetPortLinked: false },
			'local'
		);

		expect(spec.targetPort).toBe(5432);
	});

	it('should turn blank optional texts into undefined', () => {
		const spec = toTunnelSpec(
			{ ...valid, targetHost: '  ', listenAddress: '' },
			'local'
		);

		expect([spec.targetHost, spec.listenAddress]).toEqual([
			undefined,
			undefined,
		]);
	});

	it('should trim the name and parse the tags', () => {
		const spec = toTunnelSpec({ ...valid, tags: 'db, db ,web' }, 'local');

		expect([spec.name, spec.tags]).toEqual(['Adonis', ['db', 'web']]);
	});

	it('should keep the given kind', () => {
		expect(toTunnelSpec(valid, 'remote').kind).toBe('remote');
	});

	it('should throw on an invalid listen port', () => {
		expect(() => toTunnelSpec({ ...valid, listenPort: 'x' }, 'local')).toThrow(
			RangeError
		);
	});
});
