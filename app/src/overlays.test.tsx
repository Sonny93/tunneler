import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen } from '@testing-library/react';

import { Overlays } from '#/overlays';
import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';

const renderOverlays = () => {
	useDaemonStore.setState({ settings: buildSettings() });
	return renderWithGateway(
		<>
			<ModalProvider />
			<Overlays />
		</>
	);
};

describe('Overlays', () => {
	it('should render nothing visible by default', async () => {
		const { container } = renderOverlays();

		expect(await axe(container)).toHaveNoViolations();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should open the tunnel editor with the n shortcut', async () => {
		renderOverlays();

		await userEvent.keyboard('n');

		expect(
			await screen.findByRole('dialog', { name: 'Nouveau tunnel' })
		).toBeInTheDocument();
	});

	it('should open the settings drawer from the UI store', () => {
		renderOverlays();

		act(() => useUiStore.getState().openSettings());

		expect(
			screen.getByRole('dialog', { name: 'Settings' })
		).toBeInTheDocument();
	});

	it('should open the help with the ? shortcut', async () => {
		renderOverlays();

		await userEvent.keyboard('?');

		expect(
			await screen.findByRole('dialog', { name: 'Raccourcis clavier' })
		).toBeInTheDocument();
	});

	it('should show a pending prompt', async () => {
		renderOverlays();

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'credential.request',
				data: {
					promptId: 'p1',
					kind: 'password',
					alias: 'nova',
					user: 'jules',
					host: 'h',
				},
			})
		);

		expect(
			await screen.findByRole('dialog', { name: 'Mot de passe' })
		).toBeInTheDocument();
	});

	it('should not trigger shortcuts while a drawer is open', async () => {
		renderOverlays();
		act(() => useUiStore.getState().openSettings());

		await userEvent.keyboard('?');

		expect(useUiStore.getState().isHelpOpen).toBe(false);
	});

	it('should open the import drawer with the i shortcut', async () => {
		renderOverlays();

		await userEvent.keyboard('i');

		expect(
			await screen.findByRole('dialog', { name: 'Importer des tunnels' })
		).toBeInTheDocument();
	});

	it('should open the tunnel details drawer from the UI store', () => {
		useDaemonStore.setState({ tunnels: [buildTunnel()] });
		renderOverlays();

		act(() => useUiStore.getState().openTunnelDetails('tunnel-1'));

		expect(
			screen.getByRole('dialog', { name: 'Tunnel 3333 → nova:3333' })
		).toBeInTheDocument();
	});

	it('should open the host configuration drawer from the UI store', () => {
		renderOverlays();

		act(() => useUiStore.getState().openHostConfig('nova'));

		expect(
			screen.getByRole('dialog', { name: 'Configuration de nova' })
		).toBeInTheDocument();
	});

	it('should open the command palette with Ctrl+K', async () => {
		renderOverlays();

		await userEvent.keyboard('{Control>}k{/Control}');

		expect(
			await screen.findByRole('dialog', { name: 'Palette de commandes' })
		).toBeInTheDocument();
	});
});
