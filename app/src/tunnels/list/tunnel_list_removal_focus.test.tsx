import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';

import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

const NAMES_BY_ID = { a: 'Alpha', b: 'Beta', c: 'Gamma' } as const;

function renderList(ids: readonly (keyof typeof NAMES_BY_ID)[]) {
	useDaemonStore.setState({
		tunnels: ids.map((id) => buildTunnel({ id, name: NAMES_BY_ID[id] })),
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

async function deleteTunnelNamed(name: string) {
	await userEvent.click(
		screen.getByRole('button', { name: `Actions pour ${name}` })
	);
	await userEvent.click(screen.getByRole('menuitem', { name: 'Supprimer' }));
	await userEvent.click(
		await screen.findByRole('button', { name: 'Supprimer' })
	);
}

const rowSwitch = (name: string) =>
	screen.getByRole('switch', { name: new RegExp(`^Tunnel ${name},`) });

describe('TunnelList focus after removal', () => {
	it('should focus the next row switch when a middle row is deleted', async () => {
		renderList(['a', 'b', 'c']);

		await deleteTunnelNamed('Beta');

		expect(rowSwitch('Gamma')).toHaveFocus();
		expect(rowSwitch('Gamma')).toHaveAttribute('tabindex', '0');
	});

	it('should focus the previous row switch when the last row is deleted', async () => {
		renderList(['a', 'b', 'c']);

		await deleteTunnelNamed('Gamma');

		expect(rowSwitch('Beta')).toHaveFocus();
	});

	it('should focus the create button when the only row is deleted', async () => {
		renderList(['a']);

		await deleteTunnelNamed('Alpha');

		expect(
			screen.getByRole('button', { name: 'Créer un tunnel' })
		).toHaveFocus();
	});
});
