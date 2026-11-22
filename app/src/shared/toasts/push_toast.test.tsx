import { Toaster } from '@minimalstuff/ui';
import { act, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { activateLocale } from '#/i18n/activate_locale';
import { AppUIProvider } from '#/shared/app_ui_provider';
import { pushToast, type ToastTone } from '#/shared/toasts/push_toast';
import { renderWithI18n } from '#/testing/render_with_i18n';
import { getShownToasts } from '#/testing/shown_toasts';

const TONES: readonly ToastTone[] = ['info', 'success', 'warning', 'error'];

function renderToaster() {
	return renderWithI18n(
		<AppUIProvider>
			<Toaster />
		</AppUIProvider>
	);
}

async function findNotifications() {
	return screen.findByRole('region', { name: 'Notifications' });
}

describe('pushToast', () => {
	it.each(TONES)('should show a %s toast with its message', (tone) => {
		pushToast({ tone, message: 'Hello' });

		expect(getShownToasts()).toEqual([{ tone, message: 'Hello' }]);
	});

	it('should render the message in the notifications region', async () => {
		renderToaster();

		act(() => pushToast({ tone: 'success', message: 'Configuration saved' }));

		const region = await findNotifications();
		expect(
			await within(region).findByText('Configuration saved')
		).toBeInTheDocument();
	});

	it('should have no axe violations with toasts of each tone', async () => {
		renderToaster();
		act(() => {
			for (const tone of TONES) pushToast({ tone, message: `Toast ${tone}` });
		});
		await findNotifications();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it.each([
		['en', 'Close notification'],
		['fr', 'Fermer la notification'],
	] as const)(
		'should name the close button of a toast in %s',
		async (locale, expectedName) => {
			await activateLocale(locale);
			renderToaster();

			act(() => pushToast({ tone: 'error', message: 'Failure' }));

			const region = await findNotifications();
			expect(
				await within(region).findByRole('button', { name: expectedName })
			).toBeInTheDocument();
		}
	);
});
