import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { X11ProbeDto, X11ServerStartDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildSettings, buildX11Launcher } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { X11Drawer } from '#/x11/x11_drawer';

const reachable: X11ProbeDto = {
	display: ':0',
	isReachable: true,
	server: null,
};
const stopped: X11ProbeDto = {
	display: '127.0.0.1:0.0',
	isReachable: false,
	server: 'VcXsrv',
};
const missing: X11ProbeDto = { ...stopped, server: null };

function setup(probe: X11ProbeDto, x11ServerStart: X11ServerStartDto = 'ask') {
	useDaemonStore.setState({
		settings: buildSettings({ x11ServerStart }),
		x11Launchers: [buildX11Launcher()],
	});
	const gateway = createFakeGateway();
	gateway.probeX11.mockResolvedValue(probe);
	const rendered = renderWithGateway(
		<>
			<X11Drawer />
			<ModalProvider />
		</>,
		gateway
	);
	return { ...rendered, gateway };
}

const toasts = () => useToastStore.getState().toasts;

const clickStart = () =>
	userEvent.click(screen.getByRole('button', { name: 'Start X11' }));

describe('useEnsureX11Server', () => {
	beforeEach(() => {
		useUiStore.getState().openX11('nova');
		useUiStore.getState().confirmX11Host('nova');
	});

	it('should only start X11 when the X server is reachable', async () => {
		const { gateway } = setup(reachable, 'always');

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).not.toHaveBeenCalled();
		expect(toasts()).toEqual([]);
	});

	it('should start the X server before X11 when the setting is always', async () => {
		const { gateway } = setup(stopped, 'always');

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).toHaveBeenCalledWith('nova');
		expect(gateway.startX11Server.mock.invocationCallOrder[0]).toBeLessThan(
			gateway.startX11.mock.invocationCallOrder[0]
		);
	});

	it('should still start X11 and toast when the X server fails to start', async () => {
		const { gateway } = setup(stopped, 'always');
		gateway.startX11Server.mockRejectedValueOnce(
			new DaemonCommandError('x11_server', 'not found')
		);

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(toasts().map((toast) => toast.message)).toEqual([
			"Couldn't start VcXsrv: not found",
		]);
	});

	it('should ask before starting the X server when the setting is ask', async () => {
		const { gateway, container } = setup(stopped, 'ask');

		await clickStart();

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Start VcXsrv?',
		});
		expect(dialog).toHaveTextContent(
			'No X server is running on 127.0.0.1:0.0, so windows opened on nova would not appear.'
		);
		expect(gateway.startX11).not.toHaveBeenCalled();
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should start the X server then X11 when the question is confirmed', async () => {
		const { gateway } = setup(stopped, 'ask');

		await clickStart();
		await userEvent.click(
			await screen.findByRole('button', { name: 'Start VcXsrv' })
		);

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).toHaveBeenCalledWith('nova');
	});

	it('should start X11 without the X server when the question is declined', async () => {
		const { gateway } = setup(stopped, 'ask');

		await clickStart();
		await userEvent.click(
			await screen.findByRole('button', { name: 'Continue without' })
		);

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).not.toHaveBeenCalled();
	});

	it('should warn and start X11 when the setting is never', async () => {
		const { gateway } = setup(stopped, 'never');

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).not.toHaveBeenCalled();
		expect(toasts()).toEqual([
			expect.objectContaining({
				tone: 'warning',
				message:
					'No X server is running on 127.0.0.1:0.0: windows opened on nova will not appear.',
			}),
		]);
	});

	it('should warn that no X server is installed and start X11', async () => {
		const { gateway } = setup(missing, 'always');

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(gateway.startX11Server).not.toHaveBeenCalled();
		expect(toasts()).toEqual([
			expect.objectContaining({
				tone: 'warning',
				message:
					'No X server is running on 127.0.0.1:0.0: windows opened on nova will not appear. Start your X server, or on Windows install one such as VcXsrv.',
			}),
		]);
	});

	it('should not block X11 when the probe fails', async () => {
		const { gateway } = setup(reachable);
		gateway.probeX11.mockRejectedValueOnce(
			new DaemonCommandError('internal', 'boom')
		);

		await clickStart();

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(toasts()).toEqual([]);
	});

	it('should check the X server before running a launcher', async () => {
		const { gateway } = setup(stopped, 'always');

		await userEvent.click(
			screen.getByRole('button', { name: 'Launch VS Code on nova' })
		);

		await waitFor(() =>
			expect(gateway.launchX11).toHaveBeenCalledWith('nova', 'VS Code')
		);
		expect(gateway.startX11Server).toHaveBeenCalledWith('nova');
	});
});
