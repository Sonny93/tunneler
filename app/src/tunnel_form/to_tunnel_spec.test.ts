import { describe, expect, it } from 'vitest';

import {
	toTemporaryTunnelSpec,
	toTunnelSpec,
} from '#/tunnel_form/to_tunnel_spec';
import { EMPTY_TUNNEL_FORM_VALUES } from '#/tunnel_form/tunnel_form_values';

const valid = {
	...EMPTY_TUNNEL_FORM_VALUES,
	host: 'nova',
	listenPort: '3333',
	targetPort: '3333',
};

describe('toTunnelSpec', () => {
	it('should omit the target port while linked', () => {
		expect(toTunnelSpec(valid).targetPort).toBeUndefined();
	});

	it('should send the target port once unlinked', () => {
		const spec = toTunnelSpec({
			...valid,
			targetPort: '5432',
			isTargetPortLinked: false,
		});

		expect(spec.targetPort).toBe(5432);
	});

	it('should turn blank optional texts into undefined', () => {
		const spec = toTunnelSpec({
			...valid,
			targetHost: '  ',
			listenAddress: '',
		});

		expect([spec.targetHost, spec.listenAddress]).toEqual([
			undefined,
			undefined,
		]);
	});

	it('should trim the host and parse the tags', () => {
		const spec = toTunnelSpec({
			...valid,
			host: ' nova ',
			tags: 'db, db ,web',
		});

		expect([spec.host, spec.tags]).toEqual(['nova', ['db', 'web']]);
	});

	it('should keep the given kind', () => {
		expect(toTunnelSpec({ ...valid, kind: 'remote' }).kind).toBe('remote');
	});

	it('should omit the target of a dynamic tunnel', () => {
		const spec = toTunnelSpec({
			...valid,
			kind: 'dynamic',
			targetHost: 'db',
			targetPort: '5432',
			isTargetPortLinked: false,
		});

		expect([spec.targetHost, spec.targetPort]).toEqual([undefined, undefined]);
	});

	it('should keep the target of a remote tunnel', () => {
		const spec = toTunnelSpec({
			...valid,
			kind: 'remote',
			targetHost: 'db',
			targetPort: '5432',
			isTargetPortLinked: false,
		});

		expect([spec.targetHost, spec.targetPort]).toEqual(['db', 5432]);
	});

	it('should throw on an invalid listen port', () => {
		expect(() => toTunnelSpec({ ...valid, listenPort: 'x' })).toThrow(
			RangeError
		);
	});

	it('should omit the open options when none is set', () => {
		expect(toTunnelSpec(valid).open).toBeUndefined();
	});

	it('should send only the open options that differ from the defaults', () => {
		const spec = toTunnelSpec({
			...valid,
			openScheme: 'https',
			openPath: ' /admin ',
			openHost: 'ip',
		});

		expect(spec.open).toEqual({
			scheme: 'https',
			path: '/admin',
			host: 'ip',
		});
	});

	it('should not send the default http scheme', () => {
		const spec = toTunnelSpec({ ...valid, openPath: '/x' });

		expect(spec.open).toEqual({ path: '/x' });
	});

	it('should drop the open options of a remote tunnel', () => {
		const spec = toTunnelSpec({
			...valid,
			kind: 'remote',
			openPath: '/x',
		});

		expect(spec.open).toBeUndefined();
	});

	it('should send the chosen application', () => {
		expect(toTunnelSpec({ ...valid, openWith: 'remmina-vnc' }).open).toEqual({
			with: 'remmina-vnc',
		});
	});

	it('should send the browser choice explicitly', () => {
		expect(toTunnelSpec({ ...valid, openWith: 'browser' }).open).toEqual({
			with: 'browser',
		});
	});

	it('should omit the standby choice when it is inherited', () => {
		expect(toTunnelSpec(valid).standby).toBeUndefined();
	});

	it.each([
		['yes', true],
		['no', false],
	] as const)(
		'should send the standby choice "%s" as %s',
		(choice, expected) => {
			expect(toTunnelSpec({ ...valid, standby: choice }).standby).toBe(
				expected
			);
		}
	);

	it('should drop the standby choice of a remote tunnel', () => {
		const spec = toTunnelSpec({ ...valid, kind: 'remote', standby: 'yes' });

		expect(spec.standby).toBeUndefined();
	});

	it('should not carry the standby choice into a temporary tunnel', () => {
		const spec = toTemporaryTunnelSpec({ ...valid, standby: 'yes' });

		expect('standby' in spec).toBe(false);
	});
});
