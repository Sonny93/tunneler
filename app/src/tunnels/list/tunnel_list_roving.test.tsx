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
			buildTunnel({ id: 'a', listenPort: 3001, host: 'nova' }),
			buildTunnel({ id: 'b', listenPort: 3002, host: 'nova' }),
			buildTunnel({ id: 'c', listenPort: 3003, host: 'zeta' }),
		],
	});
}

const ALPHA = '3001 vers nova:3001';
const BETA = '3002 vers nova:3002';
const GAMMA = '3003 vers zeta:3003';

const rowCheckbox = (name: string) =>
	screen.getByRole('checkbox', { name: `Sélectionner ${name}` });
const rowSwitch = (name: string) =>
	screen.getByRole('switch', { name: new RegExp(`^Tunnel ${name},`) });
const actionsButton = (name: string) =>
	screen.getByRole('button', { name: `Actions pour ${name}` });

const STOPS_BEFORE_FIRST_ROW_SWITCH = 5;

const tabIntoFirstRow = async () => {
	for (
		let tabCount = 0;
		tabCount < STOPS_BEFORE_FIRST_ROW_SWITCH;
		tabCount += 1
	) {
		await userEvent.tab();
	}
};

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

		await tabIntoFirstRow();

		expect(rowSwitch(ALPHA)).toHaveFocus();
	});

	it('should reach the first actions button with a second Tab', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await tabIntoFirstRow();
		await userEvent.tab();

		expect(actionsButton(ALPHA)).toHaveFocus();
	});

	it('should reach the first row checkbox just before its switch', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		for (
			let tabCount = 0;
			tabCount < STOPS_BEFORE_FIRST_ROW_SWITCH - 1;
			tabCount += 1
		) {
			await userEvent.tab();
		}

		expect(rowCheckbox(ALPHA)).toHaveFocus();
	});

	it('should move from checkbox to checkbox with ArrowDown', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowCheckbox(ALPHA).focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowCheckbox(BETA)).toHaveFocus();
	});

	it('should move across the controls of a row with ArrowRight', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowCheckbox(ALPHA).focus();

		await userEvent.keyboard('{ArrowRight}');
		expect(rowSwitch(ALPHA)).toHaveFocus();

		await userEvent.keyboard('{ArrowRight}');
		expect(actionsButton(ALPHA)).toHaveFocus();
	});

	it('should move back across the controls of a row with ArrowLeft', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton(BETA).focus();

		await userEvent.keyboard('{ArrowLeft}');
		expect(rowSwitch(BETA)).toHaveFocus();

		await userEvent.keyboard('{ArrowLeft}');
		expect(rowCheckbox(BETA)).toHaveFocus();
	});

	it('should stay on the first control of a row with ArrowLeft', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowCheckbox(ALPHA).focus();

		await userEvent.keyboard('{ArrowLeft}');

		expect(rowCheckbox(ALPHA)).toHaveFocus();
	});

	it('should move to the next row switch with ArrowDown', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await tabIntoFirstRow();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch(BETA)).toHaveFocus();
	});

	it('should cross host groups with ArrowDown', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch(BETA).focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch(GAMMA)).toHaveFocus();
	});

	it('should move to the previous row switch with ArrowUp', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch(BETA).focus();

		await userEvent.keyboard('{ArrowUp}');

		expect(rowSwitch(ALPHA)).toHaveFocus();
	});

	it('should not wrap past the last row', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch(GAMMA).focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(rowSwitch(GAMMA)).toHaveFocus();
	});

	it('should jump to the last and first rows with End and Home', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await tabIntoFirstRow();

		await userEvent.keyboard('{End}');
		expect(rowSwitch(GAMMA)).toHaveFocus();

		await userEvent.keyboard('{Home}');
		expect(rowSwitch(ALPHA)).toHaveFocus();
	});

	it('should move from actions button to actions button', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton(ALPHA).focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(actionsButton(BETA)).toHaveFocus();
	});

	it('should not open the actions menu with the arrow keys', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton(ALPHA).focus();

		await userEvent.keyboard('{ArrowDown}');

		expect(screen.queryByRole('menu')).not.toBeInTheDocument();
	});

	it('should open the actions menu with Enter', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		actionsButton(ALPHA).focus();

		await userEvent.keyboard('{Enter}');

		expect(screen.getByRole('menu')).toBeInTheDocument();
	});

	it('should still toggle a switch with Space', async () => {
		seedTunnels();
		const { gateway } = renderWithGateway(<TunnelList />);
		await tabIntoFirstRow();

		await userEvent.keyboard(' ');

		expect(gateway.startTunnel).toHaveBeenCalledWith('a');
	});

	it('should make the focused row the only tabbable one', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		await tabIntoFirstRow();

		await userEvent.keyboard('{ArrowDown}');

		expect(
			screen.getAllByRole('switch').map((item) => item.getAttribute('tabindex'))
		).toEqual(['-1', '0', '-1']);
		expect(rowCheckbox(BETA)).toHaveAttribute('tabindex', '0');
		expect(actionsButton(BETA)).toHaveAttribute('tabindex', '0');
		expect(actionsButton(ALPHA)).toHaveAttribute('tabindex', '-1');
	});

	it('should make a clicked row active', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await userEvent.click(actionsButton(GAMMA));

		expect(actionsButton(GAMMA)).toHaveAttribute('tabindex', '0');
	});

	it('should fall back to the first row when the active row disappears', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);
		rowSwitch(GAMMA).focus();

		act(() => {
			useDaemonStore.setState({
				tunnels: useDaemonStore
					.getState()
					.tunnels.filter(({ id }) => id !== 'c'),
			});
		});

		expect(rowSwitch(ALPHA)).toHaveAttribute('tabindex', '0');
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
