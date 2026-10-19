import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { createFakeGateway } from '#/testing/fake_gateway';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelFormDrawer } from '#/tunnel_form/tunnel_form_drawer';

const CREATED_ID = 'created-tunnel';

function renderCreation() {
	useDaemonStore.setState({
		tunnels: [buildTunnel({ id: 'a', host: 'nova' })],
		settings: buildSettings({ defaultBindAddress: '127.0.0.1' }),
		hosts: ['nova'],
	});
	const gateway = createFakeGateway();
	gateway.createTunnel.mockImplementation((spec) => {
		const created = buildTunnel({ ...spec, id: CREATED_ID });
		useDaemonStore.setState({
			tunnels: [...useDaemonStore.getState().tunnels, created],
		});
		return Promise.resolve(created);
	});
	renderWithGateway(
		<>
			<main tabIndex={-1}>
				<TunnelList />
			</main>
			<TunnelFormDrawer />
		</>,
		gateway
	);
	return gateway;
}

async function createTunnelOn(host: string) {
	const user = userEvent.setup();
	useUiStore.getState().openTunnelEditor(null);
	await user.type(await screen.findByRole('combobox', { name: /^Host/ }), host);
	await user.type(
		screen.getByRole('textbox', { name: /^Port d'écoute/ }),
		'4444'
	);
	await user.click(screen.getByRole('button', { name: 'Créer' }));
}

describe('TunnelList focus after creation', () => {
	it('should focus the switch of the new tunnel when the drawer closes', async () => {
		renderCreation();

		await createTunnelOn('nova');

		await waitFor(() =>
			expect(
				screen.getByRole('switch', { name: /^Tunnel 4444 vers nova:4444,/ })
			).toHaveFocus()
		);
	});

	it('should make the new row the active roving row', async () => {
		renderCreation();

		await createTunnelOn('nova');

		await waitFor(() =>
			expect(
				screen.getByRole('switch', { name: /^Tunnel 4444 vers nova:4444,/ })
			).toHaveAttribute('tabindex', '0')
		);
	});

	it('should drop the focus request when the new tunnel is hidden by the filter', async () => {
		renderCreation();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'other' });
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'a', host: 'nova', tags: ['other'] })],
		});

		await createTunnelOn('nova');

		await waitFor(() =>
			expect(useUiStore.getState().pendingFocusTunnelId).toBeNull()
		);
	});
});
