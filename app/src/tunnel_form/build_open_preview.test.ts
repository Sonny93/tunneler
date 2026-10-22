import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildOpener, buildSettings } from '#/testing/factories';
import { buildOpenPreview } from '#/tunnel_form/build_open_preview';
import { EMPTY_TUNNEL_FORM_VALUES } from '#/tunnel_form/tunnel_form_values';

const values = {
	...EMPTY_TUNNEL_FORM_VALUES,
	host: 'nova',
	listenPort: '3333',
};
const settings = buildSettings({ defaultBindAddress: '127.0.0.1' });

const preview = (
	formValues: Parameters<typeof buildOpenPreview>[0],
	generalSettings: Parameters<typeof buildOpenPreview>[1],
	openers: Parameters<typeof buildOpenPreview>[2] = []
) => buildOpenPreview(formValues, generalSettings, openers, i18n);

describe('buildOpenPreview', () => {
	it('should preview http on localhost with the root path by default', () => {
		expect(preview(values, settings)).toBe('Opens http://localhost:3333/');
	});

	it('should give nothing while the listen port is invalid', () => {
		expect(preview({ ...values, listenPort: 'abc' }, settings)).toBeNull();
	});

	it('should use the scheme and path of the form', () => {
		expect(
			preview({ ...values, openScheme: 'https', openPath: '/admin' }, settings)
		).toBe('Opens https://localhost:3333/admin');
	});

	it('should follow the general setting when the host is inherited', () => {
		expect(preview(values, buildSettings({ openHost: 'ip' }))).toBe(
			'Opens http://127.0.0.1:3333/'
		);
	});

	it('should prefer the host of the form over the general setting', () => {
		expect(
			preview(
				{ ...values, openHost: 'localhost' },
				buildSettings({ openHost: 'ip' })
			)
		).toBe('Opens http://localhost:3333/');
	});

	it('should use the ipv6 loopback when the tunnel listens on ipv6', () => {
		expect(
			preview({ ...values, openHost: 'ip', listenAddress: '::1' }, settings)
		).toBe('Opens http://[::1]:3333/');
	});

	it('should open a specific ip as is', () => {
		expect(preview({ ...values, listenAddress: '192.0.2.120' }, settings)).toBe(
			'Opens http://192.0.2.120:3333/'
		);
	});

	it('should open a wildcard listen address through the preferred loopback', () => {
		expect(preview({ ...values, listenAddress: '0.0.0.0' }, settings)).toBe(
			'Opens http://localhost:3333/'
		);
	});

	it('should say a host name cannot be opened', () => {
		expect(preview({ ...values, listenAddress: 'example.com' }, settings)).toBe(
			"The listen address is a host name: the tunnel can't be opened."
		);
	});

	it('should preview the command line of the application chosen', () => {
		expect(
			preview({ ...values, openWith: 'remmina-vnc' }, settings, [buildOpener()])
		).toBe('Runs remmina -c vnc://localhost:3333');
	});

	it('should preview the suggested application when the choice is automatic', () => {
		expect(preview(values, settings, [buildOpener({ ports: [3333] })])).toBe(
			'Runs remmina -c vnc://localhost:3333'
		);
	});

	it('should preview the url when the browser is chosen although an application matches', () => {
		expect(
			preview({ ...values, openWith: 'browser' }, settings, [
				buildOpener({ ports: [3333] }),
			])
		).toBe('Opens http://localhost:3333/');
	});

	it('should preview the url when the chosen application is not available', () => {
		expect(preview({ ...values, openWith: 'gone' }, settings, [])).toBe(
			'Opens http://localhost:3333/'
		);
	});

	it('should quote an argument that holds spaces', () => {
		expect(
			preview({ ...values, openWith: 'x' }, settings, [
				buildOpener({ id: 'x', args: ['--title', 'my {host}'] }),
			])
		).toBe('Runs remmina --title "my localhost"');
	});
});
