import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { renderWithGateway } from '#/testing/render_with_gateway';

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'z1', name: 'Zed', host: 'zeta', tags: ['db'] }),
			buildTunnel({ id: 'b2', name: 'beta', host: 'nova', tags: ['dev'] }),
			buildTunnel({ id: 'a1', name: 'Alpha', host: 'nova', tags: ['dev'] }),
		],
	});
}

describe('TunnelList', () => {
	it('should group tunnels by sorted host with tunnels sorted by name', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		const groups = screen.getAllByRole('region');

		expect(
			groups.map((group) => group.getAttribute('aria-labelledby'))
		).toHaveLength(2);
		expect(
			screen
				.getAllByRole('heading', { level: 3 })
				.map((heading) => heading.textContent)
		).toEqual(['nova', 'zeta']);
		expect(
			within(groups[0])
				.getAllByRole('switch')
				.map((rowSwitch) => rowSwitch.getAttribute('aria-label'))
		).toEqual([
			'Tunnel Alpha, 3333 vers nova:3333, arrêté',
			'Tunnel beta, 3333 vers nova:3333, arrêté',
		]);
	});

	it('should offer the resolved configuration of each host', async () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Configuration de zeta' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'hostConfig',
			host: 'zeta',
		});
	});

	it('should title the list with the filter and the count', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		expect(
			screen.getByRole('heading', { level: 2, name: 'Tous les tunnels (3)' })
		).toBeInTheDocument();
	});

	it('should apply the tag filter', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'tag', tag: 'dev' } });
		renderWithGateway(<TunnelList />);

		expect(
			screen.getByRole('heading', { level: 2, name: 'Tag dev (2)' })
		).toBeInTheDocument();
		expect(screen.getAllByRole('switch')).toHaveLength(2);
	});

	it('should apply the host filter', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'host', host: 'zeta' } });
		renderWithGateway(<TunnelList />);

		expect(screen.getAllByRole('switch')).toHaveLength(1);
	});

	it('should show every tunnel when the filtered tag no longer exists', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'tag', tag: 'gone' } });
		renderWithGateway(<TunnelList />);

		expect(screen.getAllByRole('switch')).toHaveLength(3);
	});

	it('should offer to create a tunnel when none is configured', async () => {
		renderWithGateway(<TunnelList />);

		expect(screen.getByText('Aucun tunnel configuré.')).toBeInTheDocument();
		await userEvent.click(
			screen.getByRole('button', { name: 'Créer un tunnel' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should have no accessibility violation', async () => {
		seedTunnels();
		const { container } = renderWithGateway(<TunnelList />);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no accessibility violation when empty', async () => {
		const { container } = renderWithGateway(<TunnelList />);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should offer to rename the tag the list is filtered by', async () => {
		seedTunnels();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });
		renderWithGateway(<TunnelList />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Renommer le tag' })
		);

		expect(useUiStore.getState().tagRename).toEqual({
			tag: 'dev',
			origin: 'list',
		});
	});

	it('should not offer to rename a tag without a tag filter', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		expect(
			screen.queryByRole('button', { name: 'Renommer le tag' })
		).not.toBeInTheDocument();
	});

	it('should ask for confirmation before deleting the tag the list is filtered by', async () => {
		seedTunnels();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });
		const { gateway } = renderWithGateway(
			<>
				<ModalProvider />
				<TunnelList />
			</>
		);

		await userEvent.click(
			screen.getByRole('button', { name: 'Supprimer le tag' })
		);

		expect(
			await screen.findByRole('alertdialog', { name: 'Supprimer le tag dev ?' })
		).toBeInTheDocument();
		expect(gateway.deleteTag).not.toHaveBeenCalled();
	});

	it('should not offer to delete a tag without a tag filter', () => {
		seedTunnels();
		renderWithGateway(<TunnelList />);

		expect(
			screen.queryByRole('button', { name: 'Supprimer le tag' })
		).not.toBeInTheDocument();
	});
});
