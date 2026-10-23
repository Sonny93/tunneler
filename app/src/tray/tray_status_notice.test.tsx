import { act, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import type { TrayStatusDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { renderWithI18n } from '#/testing/render_with_i18n';
import { TrayStatusNotice } from '#/tray/tray_status_notice';

const renderNotice = (status: TrayStatusDto) => {
	useDaemonStore.setState({ tray: status });
	return renderWithI18n(
		<main tabIndex={-1}>
			<TrayStatusNotice />
		</main>
	);
};

const MISSING_HOST: TrayStatusDto = { kind: 'missingHost' };
const FAILED: TrayStatusDto = { kind: 'failed', message: 'no dbus' };

describe('TrayStatusNotice', () => {
	it.each<TrayStatusDto>([{ kind: 'active' }, { kind: 'disabled' }])(
		'should render nothing when the tray is $kind',
		(status) => {
			renderNotice(status);

			expect(screen.queryByRole('region')).not.toBeInTheDocument();
		}
	);

	it('should explain the missing notification area', () => {
		renderNotice(MISSING_HOST);

		expect(
			screen.getByRole('region', {
				name: 'No notification area detected',
			})
		).toHaveTextContent('AppIndicator and KStatusNotifierItem Support');
	});

	it('should show the failure message when the icon could not be created', () => {
		renderNotice(FAILED);

		expect(
			screen.getByRole('region', { name: 'Notification icon unavailable' })
		).toHaveTextContent('The icon could not be created: no dbus.');
	});

	it('should hide the notice and move the focus to the main region on Hide', async () => {
		renderNotice(MISSING_HOST);

		await userEvent.click(screen.getByRole('button', { name: 'Hide' }));

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
		expect(screen.getByRole('main')).toHaveFocus();
	});

	it('should show the notice again when another problem appears', async () => {
		renderNotice(MISSING_HOST);
		await userEvent.click(screen.getByRole('button', { name: 'Hide' }));

		act(() => useDaemonStore.setState({ tray: FAILED }));

		expect(
			screen.getByRole('region', { name: 'Notification icon unavailable' })
		).toBeInTheDocument();
	});

	it('should keep the notice hidden when the same problem comes back', async () => {
		renderNotice(MISSING_HOST);
		await userEvent.click(screen.getByRole('button', { name: 'Hide' }));

		act(() => useDaemonStore.setState({ tray: { kind: 'missingHost' } }));

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
	});

	it('should remember the hidden problem in the ui store', async () => {
		renderNotice(FAILED);

		await userEvent.click(screen.getByRole('button', { name: 'Hide' }));

		expect(useUiStore.getState().dismissedTrayProblem).toBe('failed');
	});

	it('should have no accessibility violation', async () => {
		const { container } = renderNotice(FAILED);

		expect(await axe(container)).toHaveNoViolations();
	});
});
