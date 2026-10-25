import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { I18nTestProvider } from '#/testing/i18n_test_provider';
import { X11ServerStatus } from '#/x11/x11_server_status';
import type { X11ServerStatus as X11ServerStatusState } from '#/x11/x11_server_status_state';

function renderStatus(status: X11ServerStatusState, isStarting = false) {
	const onStart = vi.fn();
	const rendered = render(
		<I18nTestProvider>
			<X11ServerStatus
				status={status}
				isStarting={isStarting}
				onStart={onStart}
			/>
		</I18nTestProvider>
	);
	return { ...rendered, onStart };
}

describe('X11ServerStatus', () => {
	it('should say it is checking', async () => {
		const { container } = renderStatus({ kind: 'checking' });

		expect(screen.getByRole('status')).toHaveTextContent(
			'Checking the X server…'
		);
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should say an X server answers when it is reachable', async () => {
		const { container } = renderStatus({ kind: 'reachable', display: ':0' });

		expect(screen.getByRole('status')).toHaveTextContent(
			'An X server answers on :0.'
		);
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should offer to start the installed server when unreachable', async () => {
		const { container, onStart } = renderStatus({
			kind: 'unreachable',
			display: '127.0.0.1:0.0',
			server: 'VcXsrv',
		});

		expect(screen.getByRole('status')).toHaveTextContent(
			'No X server answers on 127.0.0.1:0.0.'
		);
		await userEvent.click(screen.getByRole('button', { name: 'Start VcXsrv' }));
		expect(onStart).toHaveBeenCalledWith('VcXsrv');
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should mark the start button busy while starting', () => {
		renderStatus(
			{ kind: 'unreachable', display: ':0', server: 'VcXsrv' },
			true
		);

		expect(
			screen.getByRole('button', { name: /Start VcXsrv/ })
		).toHaveAttribute('aria-disabled', 'true');
	});

	it('should suggest installing a server when none is installed', async () => {
		const { container } = renderStatus({
			kind: 'unreachable',
			display: '127.0.0.1:0.0',
			server: null,
		});

		expect(screen.getByRole('status')).toHaveTextContent(
			'No X server answers on 127.0.0.1:0.0. Start your X server, or on Windows install one such as VcXsrv (winget install marha.VcXsrv).'
		);
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should show the reason when the probe failed', async () => {
		const { container } = renderStatus({ kind: 'failed', reason: 'boom' });

		expect(screen.getByRole('status')).toHaveTextContent(
			"Couldn't check the X server: boom."
		);
		expect(await axe(container)).toHaveNoViolations();
	});
});
