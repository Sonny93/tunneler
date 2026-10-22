import { describe, expect, it } from 'vitest';
import { ThemeToggle } from '@minimalstuff/ui';
import { screen } from '@testing-library/react';

import { activateLocale } from '#/i18n/activate_locale';
import { AppUIProvider } from '#/shared/app_ui_provider';
import { renderWithI18n } from '#/testing/render_with_i18n';

const renderThemeToggle = () =>
	renderWithI18n(
		<AppUIProvider>
			<ThemeToggle />
		</AppUIProvider>
	);

describe('AppUIProvider', () => {
	it('should give the lib components English labels by default', () => {
		renderThemeToggle();

		expect(
			screen.getByRole('button', { name: /^Theme: .+, change$/ })
		).toBeInTheDocument();
	});

	it('should give the lib components French labels when French is active', async () => {
		await activateLocale('fr');

		renderThemeToggle();

		expect(
			screen.getByRole('button', { name: /^Thème : .+, changer$/ })
		).toBeInTheDocument();
	});
});
