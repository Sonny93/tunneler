import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import {
	buildSettings,
	buildX11Anchor,
	buildX11Launcher,
} from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { X11Drawer } from '#/x11/x11_drawer';

const renderDrawer = (gateway = createFakeGateway()) =>
	renderWithGateway(
		<>
			<X11Drawer />
			<ModalProvider />
		</>,
		gateway
	);

const toastMessages = () =>
	useToastStore.getState().toasts.map((toast) => toast.message);

describe('X11Drawer', () => {
	beforeEach(() => {
		useUiStore.getState().openX11('nova');
	});

	it('should render nothing while another panel is open', () => {
		useUiStore.getState().openSettings();
		renderDrawer();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should title the dialog with the host', () => {
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'X11: nova' })
		).toBeInTheDocument();
	});

	it('should show the security warning as a visible note', () => {
		renderDrawer();

		expect(screen.getByRole('note', { name: 'Security' })).toHaveTextContent(
			'trusted hosts'
		);
	});

	it('should show a stopped anchor with a start button', () => {
		renderDrawer();

		expect(screen.getByText('Stopped')).toBeInTheDocument();
		expect(
			screen.getByRole('button', { name: 'Start X11' })
		).toBeInTheDocument();
	});

	it('should show the configured local display', () => {
		useDaemonStore.setState({
			settings: buildSettings({ x11Display: '192.0.2.5:1.0' }),
		});
		renderDrawer();

		expect(screen.getByText(/Local display:/)).toHaveTextContent(
			'Local display: 192.0.2.5:1.0 (general setting)'
		);
	});

	it('should show the display overridden for the host before the general setting', () => {
		useDaemonStore.setState({
			settings: buildSettings({ x11Display: '192.0.2.5:1.0' }),
			x11HostDisplays: { nova: ':7' },
		});
		renderDrawer();

		expect(screen.getByText(/Local display:/)).toHaveTextContent(
			'Local display: :7 (this host, in tunnels.toml)'
		);
	});

	it('should show the DISPLAY fallback when no local display is configured', () => {
		useDaemonStore.setState({ settings: buildSettings() });
		renderDrawer();

		expect(screen.getByText(/Local display:/)).toHaveTextContent(
			'Local display: DISPLAY variable (127.0.0.1:0.0 on Windows) (general setting)'
		);
	});

	it('should show no local display while the settings are not loaded', () => {
		useDaemonStore.setState({ settings: null });
		renderDrawer();

		expect(screen.queryByText(/Local display:/)).not.toBeInTheDocument();
	});

	it('should show the message of a failed anchor', () => {
		useDaemonStore.setState({
			x11Anchors: [
				buildX11Anchor({ state: { kind: 'failed', message: 'refused' } }),
			],
		});
		renderDrawer();

		expect(screen.getByText('refused')).toBeInTheDocument();
	});

	it('should show the attempt of a reconnecting anchor', () => {
		useDaemonStore.setState({
			x11Anchors: [
				buildX11Anchor({
					state: { kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
				}),
			],
		});
		renderDrawer();

		expect(screen.getByText('Reconnecting (attempt 2)')).toBeInTheDocument();
	});

	it('should ask for confirmation before the first start', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Start X11' }));

		expect(
			await screen.findByRole('alertdialog', {
				name: 'Enable X11 for nova?',
			})
		).toBeInTheDocument();
		expect(gateway.startX11).not.toHaveBeenCalled();
	});

	it('should start X11 once confirmed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Start X11' }));
		await user.click(await screen.findByRole('button', { name: 'Enable X11' }));

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
	});

	it('should not start X11 when the confirmation is cancelled', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Start X11' }));
		await user.click(await screen.findByRole('button', { name: 'Cancel' }));

		expect(gateway.startX11).not.toHaveBeenCalled();
	});

	it('should not ask again once the host was confirmed', async () => {
		useUiStore.getState().confirmX11Host('nova');
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Start X11' }));

		await waitFor(() => expect(gateway.startX11).toHaveBeenCalledWith('nova'));
		expect(screen.queryByText('Enable X11 for nova?')).not.toBeInTheDocument();
	});

	it('should toast a start failure', async () => {
		useUiStore.getState().confirmX11Host('nova');
		const gateway = createFakeGateway();
		gateway.startX11.mockRejectedValueOnce(
			new DaemonCommandError('x11_unreachable', 'unreachable')
		);
		renderDrawer(gateway);

		await userEvent.click(screen.getByRole('button', { name: 'Start X11' }));

		await waitFor(() =>
			expect(toastMessages()).toEqual([
				"Couldn't start X11 for nova: unreachable",
			])
		);
	});

	it('should stop an active anchor without confirmation', async () => {
		useDaemonStore.setState({ x11Anchors: [buildX11Anchor()] });
		const { gateway } = renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: 'Stop X11' }));

		await waitFor(() => expect(gateway.stopX11).toHaveBeenCalledWith('nova'));
	});

	it('should not show the display copy while the anchor is not active', () => {
		renderDrawer();

		expect(screen.queryByText(/export DISPLAY/)).not.toBeInTheDocument();
	});

	it('should show the remote display and copy the export command', async () => {
		useDaemonStore.setState({ x11Anchors: [buildX11Anchor()] });
		const user = userEvent.setup();
		renderDrawer();
		const writeText = vi.spyOn(navigator.clipboard, 'writeText');

		await user.click(
			screen.getByRole('button', {
				name: 'Copy export DISPLAY=localhost:10.0',
			})
		);

		expect(writeText).toHaveBeenCalledWith('export DISPLAY=localhost:10.0');
		expect(toastMessages()).toEqual(['Command copied']);
	});

	it('should toast an error when the copy fails', async () => {
		useDaemonStore.setState({ x11Anchors: [buildX11Anchor()] });
		const user = userEvent.setup();
		renderDrawer();
		vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(
			new Error('no')
		);

		await user.click(screen.getByRole('button', { name: /^Copy export/ }));

		expect(useToastStore.getState().toasts[0]?.tone).toBe('error');
	});

	it('should list only the launchers of the host', () => {
		useDaemonStore.setState({
			x11Launchers: [
				buildX11Launcher(),
				buildX11Launcher({ host: 'other', name: 'Autre' }),
			],
		});
		renderDrawer();

		expect(screen.getAllByRole('listitem')).toHaveLength(1);
		expect(screen.getByText('code --no-sandbox')).toBeInTheDocument();
	});

	it('should launch a launcher and toast the success', async () => {
		useDaemonStore.setState({ x11Launchers: [buildX11Launcher()] });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Launch VS Code on nova' })
		);

		await waitFor(() =>
			expect(gateway.launchX11).toHaveBeenCalledWith('nova', 'VS Code')
		);
		await waitFor(() =>
			expect(toastMessages()).toEqual(['VS Code launched on nova'])
		);
	});

	it('should toast a launch failure', async () => {
		useDaemonStore.setState({ x11Launchers: [buildX11Launcher()] });
		const gateway = createFakeGateway();
		gateway.launchX11.mockRejectedValueOnce(
			new DaemonCommandError('not_found', 'unknown launcher')
		);
		renderDrawer(gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Launch VS Code on nova' })
		);

		await waitFor(() =>
			expect(toastMessages()).toEqual([
				"Couldn't launch VS Code on nova: unknown launcher",
			])
		);
	});

	it('should explain how to declare launchers when there is none', () => {
		renderDrawer();

		expect(screen.getByText(/\[\[x11_launchers\]\]/)).toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		useDaemonStore.setState({
			x11Anchors: [buildX11Anchor()],
			x11Launchers: [buildX11Launcher()],
			settings: buildSettings({ x11Display: '192.0.2.5:1.0' }),
		});
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
