import { describe, expect, it } from 'vitest';

import { buildSettings } from '#/testing/factories';
import { buildOpenPreview } from '#/tunnel_form/build_open_preview';
import { EMPTY_TUNNEL_FORM_VALUES } from '#/tunnel_form/tunnel_form_values';

const values = {
	...EMPTY_TUNNEL_FORM_VALUES,
	host: 'nova',
	listenPort: '3333',
};
const settings = buildSettings({ defaultBindAddress: '127.0.0.1' });

describe('buildOpenPreview', () => {
	it('should preview http on localhost with the root path by default', () => {
		expect(buildOpenPreview(values, settings)).toBe(
			'Ouvre http://localhost:3333/'
		);
	});

	it('should give nothing while the listen port is invalid', () => {
		expect(
			buildOpenPreview({ ...values, listenPort: 'abc' }, settings)
		).toBeNull();
	});

	it('should use the scheme and path of the form', () => {
		expect(
			buildOpenPreview(
				{ ...values, openScheme: 'https', openPath: '/admin' },
				settings
			)
		).toBe('Ouvre https://localhost:3333/admin');
	});

	it('should follow the general setting when the host is inherited', () => {
		expect(buildOpenPreview(values, buildSettings({ openHost: 'ip' }))).toBe(
			'Ouvre http://127.0.0.1:3333/'
		);
	});

	it('should prefer the host of the form over the general setting', () => {
		expect(
			buildOpenPreview(
				{ ...values, openHost: 'localhost' },
				buildSettings({ openHost: 'ip' })
			)
		).toBe('Ouvre http://localhost:3333/');
	});

	it('should use the ipv6 loopback when the tunnel listens on ipv6', () => {
		expect(
			buildOpenPreview(
				{ ...values, openHost: 'ip', listenAddress: '::1' },
				settings
			)
		).toBe('Ouvre http://[::1]:3333/');
	});

	it('should open a specific ip as is', () => {
		expect(
			buildOpenPreview({ ...values, listenAddress: '192.0.2.120' }, settings)
		).toBe('Ouvre http://192.0.2.120:3333/');
	});

	it('should open a wildcard listen address through the preferred loopback', () => {
		expect(
			buildOpenPreview({ ...values, listenAddress: '0.0.0.0' }, settings)
		).toBe('Ouvre http://localhost:3333/');
	});

	it('should say a host name cannot be opened', () => {
		expect(
			buildOpenPreview({ ...values, listenAddress: 'example.com' }, settings)
		).toBe(
			"L'adresse d'écoute est un nom d'hôte : le tunnel ne peut pas être ouvert."
		);
	});
});
