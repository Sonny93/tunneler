import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor } from '@testing-library/react';

import { QuitButton } from '#/quit/quit_button';
import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { useQuitRequestListener } from '#/quit/use_quit_request_listener';

function QuitHarness() {
	useQuitRequestListener();
	return <QuitButton />;
}

function renderQuit(gateway = createFakeGateway()) {
	return renderWithGateway(
		<>
			<ModalProvider />
			<QuitHarness />
		</>,
		gateway
	);
}

const tunnelsRunning = (count: number) =>
	new DaemonCommandError('tunnels_running', 'tunnels running', count);

describe('QuitButton', () => {
	it('should have no axe violations', async () => {
		const { container } = renderQuit();

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should quit without confirmation when no tunnel runs', async () => {
		const user = userEvent.setup();
		const { gateway } = renderQuit();

		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		expect(gateway.quit.mock.calls).toEqual([[false]]);
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
	});

	it('should ask for confirmation with the running count and no axe violations when tunnels run', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.quit.mockRejectedValueOnce(tunnelsRunning(2));
		renderQuit(gateway);

		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Quit Tunneler?',
		});
		expect(dialog).toHaveTextContent(
			'2 running tunnels or X11 sessions will be stopped.'
		);
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should force the quit once confirmed', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.quit.mockRejectedValueOnce(tunnelsRunning(2));
		renderQuit(gateway);
		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		await user.click(
			await screen.findByRole('button', { name: 'Stop everything and quit' })
		);

		await waitFor(() =>
			expect(gateway.quit.mock.calls).toEqual([[false], [true]])
		);
	});

	it('should not quit again when the confirmation is cancelled', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.quit.mockRejectedValueOnce(tunnelsRunning(2));
		renderQuit(gateway);
		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		await user.click(await screen.findByRole('button', { name: 'Cancel' }));

		await waitFor(() =>
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		);
		expect(gateway.quit.mock.calls).toEqual([[false]]);
	});

	it('should toast any other failure', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.quit.mockRejectedValueOnce(
			new DaemonCommandError('io_error', 'boom')
		);
		renderQuit(gateway);

		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				"Couldn't quit: boom"
			)
		);
	});

	it('should toast a failing forced quit', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.quit
			.mockRejectedValueOnce(tunnelsRunning(1))
			.mockRejectedValueOnce(new DaemonCommandError('io_error', 'boom'));
		renderQuit(gateway);
		await user.click(screen.getByRole('button', { name: 'Quit Tunneler' }));

		await user.click(
			await screen.findByRole('button', { name: 'Stop everything and quit' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toHaveLength(1)
		);
	});
});

describe('useQuitRequestListener', () => {
	const runningTunnels = [
		buildTunnel({ id: 'a', state: { kind: 'active' } }),
		buildTunnel({
			id: 'b',
			state: { kind: 'reconnecting', attempt: 1, nextRetryAt: 0 },
		}),
		buildTunnel({ id: 'c', state: { kind: 'stopped' } }),
	];

	it('should confirm with the running tunnels count and clear the request when a quit is requested', async () => {
		useDaemonStore.setState({ tunnels: runningTunnels });
		renderQuit();

		act(() => useDaemonStore.setState({ isQuitRequested: true }));

		expect(await screen.findByRole('alertdialog')).toHaveTextContent(
			'2 running tunnels or X11 sessions will be stopped.'
		);
		expect(useDaemonStore.getState().isQuitRequested).toBe(false);
	});

	it('should force the quit once the requested quit is confirmed', async () => {
		useDaemonStore.setState({ tunnels: runningTunnels });
		const user = userEvent.setup();
		const { gateway } = renderQuit();
		act(() => useDaemonStore.setState({ isQuitRequested: true }));

		await user.click(
			await screen.findByRole('button', { name: 'Stop everything and quit' })
		);

		await waitFor(() => expect(gateway.quit.mock.calls).toEqual([[true]]));
	});

	it('should quit directly when no tunnel runs', async () => {
		const { gateway } = renderQuit();

		act(() => useDaemonStore.setState({ isQuitRequested: true }));

		await waitFor(() => expect(gateway.quit.mock.calls).toEqual([[false]]));
	});
});
