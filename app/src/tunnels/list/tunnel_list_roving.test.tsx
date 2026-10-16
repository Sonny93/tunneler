import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { act, screen } from '@testing-library/react';

import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { renderWithGateway } from '#/testing/render_with_gateway';

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'a', name: 'Alpha', host: 'nova' }),
			buildTunnel({ id: 'b', name: 'Beta', host: 'nova' }),
			buildTunnel({ id: 'c', name: 'Gamma', host: 'zeta' }),
		],
	});
}

const rowSwitch = (name: string) =>
	screen.getByRole('switch', { name: new RegExp(`^Tunnel ${name},`) });
const actionsButton = (name: string) =>
	screen.getByRole('button', { name: `Actions pour ${name}` });

describe('TunnelList roving focus', () => {
	it('should make only the first row switch tabbable initially', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		expect(
			screen.getAllByRole('switch').map((item) => item.getAttribute('tabindex'))
		).toEqual(['0', '-1', '-1']);
	});

	it('should enter the list on the first row switch with Tab', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await userEvent.tab();

		expect(rowSwitch('Alpha')).toHaveFocus();
	});

	it('should reach the first actions button with a second Tab', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await userEvent.tab();
		await userEvent.tab();

		expect(actionsButton('Alpha')).toHaveFocus();
	});

	it('should move to the next row switch with ArrowDown', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await userEvent.tab();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch('Beta')).toHaveFocus();
	});

	it('should cross host groups with ArrowDown', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch('Beta').focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch('Gamma')).toHaveFocus();
	});

	it('should move to the previous row switch with ArrowUp', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch('Beta').focus();

		await userEvent.keyboard('{ArrowUp}');

		expect(rowSwitch('Alpha')).toHaveFocus();
	});

	it('should not wrap past the last row', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch('Gamma').focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch('Gamma')).toHaveFocus();
	});

	it('should jump to the last and first rows with End and Home', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await userEvent.tab();

		await userEvent.keyboard('{End}');
		expect(rowSwitch('Gamma')).toHaveFocus();

		await userEvent.keyboard('{Home}');
		expect(rowSwitch('Alpha')).toHaveFocus();
	});

	it('should move from actions button to actions button', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton('Alpha').focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(actionsButton('Beta')).toHaveFocus();
	});

	it('should not open the actions menu with the arrow keys', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton('Alpha').focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(screen.queryByRole('menu')).not.toBeInTheDocument();
	});

	it('should open the actions menu with Enter', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton('Alpha').focus();

		await userEvent.keyboard('{Enter}');

		expect(screen.getByRole('menu')).toBeInTheDocument();
	});

	it('should still toggle a switch with Space', async () => {
		seedTunnels();
		const { gateway } = renderWithGateway(<TunnelList />);
		await userEvent.tab();

		await userEvent.keyboard(' ');

		expect(gateway.startTunnel).toHaveBeenCalledWith('a');
	});

	it('should make the focused row the only tabbable one', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await userEvent.tab();

		await userEvent.keyboard('{ArrowDown}');

		expect(
			screen.getAllByRole('switch').map((item) => item.getAttribute('tabindex'))
		).toEqual(['-1', '0', '-1']);
		expect(actionsButton('Beta')).toHaveAttribute('tabindex', '0');
		expect(actionsButton('Alpha')).toHaveAttribute('tabindex', '-1');
	});

	it('should make a clicked row active', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await userEvent.click(actionsButton('Gamma'));

		expect(actionsButton('Gamma')).toHaveAttribute('tabindex', '0');
	});

	it('should fall back to the first row when the active row disappears', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch('Gamma').focus();

		act(() => {
			useDaemonStore.setState({
				tunnels: useDaemonStore
					.getState()
					.tunnels.filter(({ id }) => id !== 'c'),
			});
		});

		expect(rowSwitch('Alpha')).toHaveAttribute('tabindex', '0');
	});

	it('should describe the arrow key navigation on each list', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		const [firstList] = screen.getAllByRole('list');

		expect(firstList).toHaveAccessibleDescription(
			'Utilisez les flèches haut et bas pour passer d’un tunnel à l’autre.'
		);
	});
});
