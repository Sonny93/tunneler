import { describe, expect, it } from 'vitest';
import { useLingui } from '@lingui/react/macro';
import { act, render, screen } from '@testing-library/react';

import { buildSettings } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { LocaleProvider } from '#/i18n/locale_provider';

function Greeting() {
	const { t } = useLingui();
	return <p>{t`Settings`}</p>;
}

const renderProvider = () =>
	render(
		<LocaleProvider>
			<Greeting />
		</LocaleProvider>
	);

describe('LocaleProvider', () => {
	it('should render the children in the locale of the language setting', async () => {
		useDaemonStore.setState({ settings: buildSettings({ language: 'fr' }) });

		renderProvider();

		expect(await screen.findByText('Paramètres')).toBeInTheDocument();
		expect(document.documentElement.lang).toBe('fr');
	});

	it('should follow the language setting when it changes', async () => {
		renderProvider();
		await screen.findByText('Settings');

		act(() =>
			useDaemonStore.setState({ settings: buildSettings({ language: 'fr' }) })
		);

		expect(await screen.findByText('Paramètres')).toBeInTheDocument();
	});

	it('should follow the system languages when the setting is system', async () => {
		useDaemonStore.setState({
			settings: buildSettings({ language: 'system' }),
		});
		const languages = Object.getOwnPropertyDescriptor(
			Navigator.prototype,
			'languages'
		);
		Object.defineProperty(navigator, 'languages', {
			value: ['fr-FR'],
			configurable: true,
		});

		renderProvider();

		expect(await screen.findByText('Paramètres')).toBeInTheDocument();
		Reflect.deleteProperty(navigator, 'languages');
		expect(languages).toBeDefined();
	});
});
