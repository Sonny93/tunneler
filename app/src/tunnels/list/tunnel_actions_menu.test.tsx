import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';

import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelActionsMenu } from '#/tunnels/list/tunnel_actions_menu';

function renderMenu(
	overrides: Parameters<typeof buildTunnel>[0] = {},
	gateway = createFakeGateway()
) {
	return renderWithGateway(
		<>
			<TunnelActionsMenu tunnel={buildTunnel(overrides)} isActiveRow />
			<ModalProvider />
		</>,
		gateway
	);
}

async function openMenu() {
	await userEvent.click(
		screen.getByRole('button', { name: 'Actions pour Adonis' })
	);
}

describe('TunnelActionsMenu', () => {
	it('should offer edit, restart and delete', async () => {
		renderMenu({ state: { kind: 'active' } });

		await openMenu();

		expect(
			screen.getAllByRole('menuitem').map((item) => item.textContent)
		).toEqual(['Modifier', 'Redémarrer', 'Supprimer']);
	});

	it('should open the editor on the tunnel when editing', async () => {
		renderMenu();
		await openMenu();

		await userEvent.click(screen.getByRole('menuitem', { name: 'Modifier' }));

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: 'tunnel-1',
		});
	});

	it('should disable restart when the tunnel is stopped', async () => {
		renderMenu();

		await openMenu();

		expect(screen.getByRole('menuitem', { name: 'Redémarrer' })).toBeDisabled();
	});

	it('should restart a running tunnel', async () => {
		const { gateway } = renderMenu({ state: { kind: 'active' } });
		await openMenu();

		await userEvent.click(screen.getByRole('menuitem', { name: 'Redémarrer' }));

		expect(gateway.restartTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should ask for confirmation before deleting', async () => {
		const { gateway } = renderMenu();
		await openMenu();

		await userEvent.click(screen.getByRole('menuitem', { name: 'Supprimer' }));

		expect(
			await screen.findByRole('alertdialog', {
				name: 'Supprimer le tunnel Adonis ?',
			})
		).toBeInTheDocument();
		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
	});

	it('should delete the tunnel and confirm with a toast when confirmed', async () => {
		const { gateway } = renderMenu();
		await openMenu();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Supprimer' }));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Supprimer' })
		);

		expect(gateway.deleteTunnel).toHaveBeenCalledWith('tunnel-1');
		expect(useToastStore.getState().toasts[0]?.message).toBe(
			'Tunnel Adonis supprimé'
		);
	});

	it('should keep the tunnel when the deletion is cancelled', async () => {
		const { gateway } = renderMenu();
		await openMenu();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Supprimer' }));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Annuler' })
		);

		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
	});

	it('should push an error toast when the deletion fails', async () => {
		const gateway = createFakeGateway();
		gateway.deleteTunnel.mockRejectedValue(new Error('boom'));
		renderMenu({}, gateway);
		await openMenu();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Supprimer' }));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Supprimer' })
		);

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			'Suppression impossible'
		);
	});

	it('should have no accessibility violation when open', async () => {
		const { container } = renderMenu({ state: { kind: 'active' } });
		await openMenu();

		expect(await axe(container)).toHaveNoViolations();
	});
});
