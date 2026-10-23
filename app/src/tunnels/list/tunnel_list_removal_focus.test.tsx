import { ModalProvider } from '@minimalstuff/ui';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { useDaemonStore } from '#/state/daemon_store';
import { buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelList } from '#/tunnels/list/tunnel_list';

const PORTS_BY_ID = { a: 3001, b: 3002, c: 3003 } as const;
const ALPHA = '3001 to nova:3001';
const BETA = '3002 to nova:3002';
const GAMMA = '3003 to nova:3003';

function renderList(ids: readonly (keyof typeof PORTS_BY_ID)[]) {
	useDaemonStore.setState({
		tunnels: ids.map((id) => buildTunnel({ id, listenPort: PORTS_BY_ID[id] })),
	});
	const gateway = createFakeGateway();
	gateway.deleteTunnel.mockImplementation((id) => {
		useDaemonStore.setState({
			tunnels: useDaemonStore.getState().tunnels.filter((t) => t.id !== id),
		});
		return Promise.resolve();
	});
	renderWithGateway(
		<>
			<TunnelList />
			<ModalProvider />
		</>,
		gateway
	);
}

async function deleteTunnelLabeled(name: string) {
	await userEvent.click(
		screen.getByRole('button', { name: `Actions for ${name}` })
	);
	await userEvent.click(screen.getByRole('menuitem', { name: 'Delete' }));
	await userEvent.click(await screen.findByRole('button', { name: 'Delete' }));
}

const rowSwitch = (name: string) =>
	screen.getByRole('switch', { name: new RegExp(`^Tunnel ${name},`) });

describe('TunnelList focus after removal', () => {
	it('should focus the next row switch when a middle row is deleted', async () => {
		renderList(['a', 'b', 'c']);

		await deleteTunnelLabeled(BETA);

		expect(rowSwitch(GAMMA)).toHaveFocus();
		expect(rowSwitch(GAMMA)).toHaveAttribute('tabindex', '0');
	});

	it('should focus the previous row switch when the last row is deleted', async () => {
		renderList(['a', 'b', 'c']);

		await deleteTunnelLabeled(GAMMA);

		expect(rowSwitch(BETA)).toHaveFocus();
	});

	it('should focus the create button when the only row is deleted', async () => {
		renderList(['a']);

		await deleteTunnelLabeled(ALPHA);

		expect(
			screen.getByRole('button', { name: 'Create a tunnel' })
		).toHaveFocus();
	});
});
