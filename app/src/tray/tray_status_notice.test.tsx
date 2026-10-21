import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { act, render, screen } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import type { TrayStatusDto } from '#/daemon/protocol';
import { TrayStatusNotice } from '#/tray/tray_status_notice';

const renderNotice = (status: TrayStatusDto) => {
	useDaemonStore.setState({ tray: status });
	return render(
		<main tabIndex={-1}>
			<TrayStatusNotice />
		</main>
	);
};

const MISSING_HOST: TrayStatusDto = { kind: 'missingHost' };
const FAILED: TrayStatusDto = { kind: 'failed', message: 'pas de dbus' };

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
				name: 'Aucune zone de notification détectée',
			})
		).toHaveTextContent('AppIndicator and KStatusNotifierItem Support');
	});

	it('should show the failure message when the icon could not be created', () => {
		renderNotice(FAILED);

		expect(
			screen.getByRole('region', { name: 'Icône de notification indisponible' })
		).toHaveTextContent('L’icône n’a pas pu être créée : pas de dbus.');
	});

	it('should hide the notice and move the focus to the main region on Masquer', async () => {
		renderNotice(MISSING_HOST);

		await userEvent.click(screen.getByRole('button', { name: 'Masquer' }));

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
		expect(screen.getByRole('main')).toHaveFocus();
	});

	it('should show the notice again when another problem appears', async () => {
		renderNotice(MISSING_HOST);
		await userEvent.click(screen.getByRole('button', { name: 'Masquer' }));

		act(() => useDaemonStore.setState({ tray: FAILED }));

		expect(
			screen.getByRole('region', { name: 'Icône de notification indisponible' })
		).toBeInTheDocument();
	});

	it('should keep the notice hidden when the same problem comes back', async () => {
		renderNotice(MISSING_HOST);
		await userEvent.click(screen.getByRole('button', { name: 'Masquer' }));

		act(() => useDaemonStore.setState({ tray: { kind: 'missingHost' } }));

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
	});

	it('should remember the hidden problem in the ui store', async () => {
		renderNotice(FAILED);

		await userEvent.click(screen.getByRole('button', { name: 'Masquer' }));

		expect(useUiStore.getState().dismissedTrayProblem).toBe('failed');
	});

	it('should have no accessibility violation', async () => {
		const { container } = renderNotice(FAILED);

		expect(await axe(container)).toHaveNoViolations();
	});
});
