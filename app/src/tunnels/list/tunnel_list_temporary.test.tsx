import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelList } from '#/tunnels/list/tunnel_list';

const SAVED = '3001 to nova:3001';
const TEMPORARY = '5000 to nova:5000';
const TEMPORARY_ARROW = '5000 → nova:5000';

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'saved', listenPort: 3001 }),
			buildTunnel({
				id: 'temporary',
				listenPort: 5000,
				isTemporary: true,
				state: { kind: 'active' },
			}),
		],
	});
}

function renderList() {
	seedTunnels();
	const gateway = createFakeGateway();
	gateway.stopTunnel.mockImplementation((id) => {
		useDaemonStore.setState({
			tunnels: useDaemonStore.getState().tunnels.filter((t) => t.id !== id),
		});
		return Promise.resolve();
	});
	gateway.persistTunnel.mockImplementation((id) => {
		useDaemonStore.setState({
			tunnels: useDaemonStore
				.getState()
				.tunnels.map((t) => (t.id === id ? { ...t, isTemporary: false } : t)),
		});
		return Promise.resolve();
	});
	return renderWithGateway(
		<>
			<TunnelList />
			<ModalProvider />
		</>,
		gateway
	);
}

const openRowMenu = (label: string) =>
	userEvent.click(screen.getByRole('button', { name: `Actions for ${label}` }));

describe('TunnelList with a temporary tunnel', () => {
	it('should show the Temporary badge on the temporary row only', () => {
		renderList();

		expect(screen.getAllByText('Temporary')).toHaveLength(1);
	});

	it('should put the temporary state in the accessible name of the switch', () => {
		renderList();

		expect(
			screen.getByRole('switch', {
				name: 'Tunnel 5000 to nova:5000, temporary, active',
			})
		).toBeChecked();
	});

	it('should have no axe violations', async () => {
		const { container } = renderList();

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should hide Edit and Delete and offer to save and to stop in the menu', async () => {
		renderList();

		await openRowMenu(TEMPORARY);

		expect(
			screen.getAllByRole('menuitem').map((item) => item.textContent)
		).toEqual([
			'Details',
			'Open in the browser',
			'Save tunnel',
			'Restart',
			'Stop and remove',
		]);
	});

	it('should keep Edit and Delete on a saved tunnel', async () => {
		renderList();

		await openRowMenu(SAVED);

		expect(
			screen.queryByRole('menuitem', { name: 'Save tunnel' })
		).not.toBeInTheDocument();
		expect(
			screen.getByRole('menuitem', { name: 'Delete' })
		).toBeInTheDocument();
	});

	it('should save the tunnel, toast and drop the badge when saving', async () => {
		const { gateway } = renderList();
		await openRowMenu(TEMPORARY);

		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Save tunnel' })
		);

		expect(gateway.persistTunnel).toHaveBeenCalledWith('temporary');
		await waitFor(() =>
			expect(screen.queryByText('Temporary')).not.toBeInTheDocument()
		);
		expect(useToastStore.getState().toasts[0]?.message).toBe(
			`Tunnel ${TEMPORARY_ARROW} saved`
		);
	});

	it('should offer to save from the context menu too', async () => {
		renderList();
		const row = screen
			.getByRole('switch', { name: /^Tunnel 5000 to/ })
			.closest('li');

		await userEvent.pointer({
			keys: '[MouseRight]',
			target: within(row as HTMLElement).getByText(TEMPORARY_ARROW),
		});

		expect(
			await screen.findByRole('menuitem', { name: 'Save tunnel' })
		).toBeInTheDocument();
	});

	it('should stop and remove the tunnel, announce it and move focus when the switch is turned off', async () => {
		const { gateway } = renderList();

		await userEvent.click(
			screen.getByRole('switch', { name: /^Tunnel 5000 to/ })
		);

		expect(gateway.stopTunnel).toHaveBeenCalledWith('temporary');
		await waitFor(() =>
			expect(useAnnouncerStore.getState().polite.message).toBe(
				`Temporary tunnel ${TEMPORARY_ARROW} stopped and removed`
			)
		);
		await waitFor(() =>
			expect(
				screen.getByRole('switch', { name: /^Tunnel 3001 to/ })
			).toHaveFocus()
		);
	});

	it('should stop and remove a failed temporary tunnel from the menu', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({
					id: 'temporary',
					listenPort: 5000,
					isTemporary: true,
					state: { kind: 'failed', reason: { kind: 'connect', message: 'x' } },
				}),
			],
		});
		const gateway = createFakeGateway();
		renderWithGateway(<TunnelList />, gateway);
		await openRowMenu(TEMPORARY);

		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Stop and remove' })
		);

		expect(gateway.stopTunnel).toHaveBeenCalledWith('temporary');
	});
});

describe('TunnelList bulk actions with a temporary tunnel', () => {
	const selectBoth = async () => {
		await userEvent.click(screen.getByRole('checkbox', { name: 'Select all' }));
	};
	const toolbarButton = (name: string) =>
		within(
			screen.getByRole('toolbar', { name: 'Actions on the selection' })
		).getByRole('button', { name });

	it('should disable Tags… when only temporary tunnels are selected', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'temporary', isTemporary: true })],
		});
		renderWithGateway(<TunnelList />);

		await userEvent.click(screen.getByRole('checkbox', { name: 'Select all' }));

		expect(toolbarButton('Tags…')).toBeDisabled();
	});

	it('should keep Tags… enabled when saved tunnels are selected with it', async () => {
		renderList();

		await selectBoth();

		expect(toolbarButton('Tags…')).toBeEnabled();
	});

	it('should stop the running tunnels and remove the temporary ones when stopping the selection', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({
					id: 'saved',
					listenPort: 3001,
					state: { kind: 'active' },
				}),
				buildTunnel({
					id: 'temporary',
					listenPort: 5000,
					isTemporary: true,
					state: { kind: 'active' },
				}),
			],
		});
		const gateway = createFakeGateway();
		renderWithGateway(<TunnelList />, gateway);
		await selectBoth();

		await userEvent.click(toolbarButton('Stop'));

		await waitFor(() => expect(gateway.stopTunnel).toHaveBeenCalledTimes(2));
	});

	it('should enable stopping when the selection holds a failed temporary tunnel', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({
					id: 'temporary',
					isTemporary: true,
					state: { kind: 'failed', reason: { kind: 'connect', message: 'x' } },
				}),
			],
		});
		renderWithGateway(<TunnelList />);

		await userEvent.click(screen.getByRole('checkbox', { name: 'Select all' }));

		expect(toolbarButton('Stop')).toBeEnabled();
	});
});
