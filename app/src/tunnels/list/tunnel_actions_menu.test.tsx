import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
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
	it('should offer details, browser, edit, restart and delete', async () => {
		renderMenu({ state: { kind: 'active' } });

		await openMenu();

		expect(
			screen.getAllByRole('menuitem').map((item) => item.textContent)
		).toEqual([
			'Détails',
			'Ouvrir dans le navigateur',
			'Modifier',
			'Redémarrer',
			'Supprimer',
		]);
	});

	it('should not offer the browser for a remote forward', async () => {
		renderMenu({ kind: 'remote' });

		await openMenu();

		expect(
			screen.queryByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		).not.toBeInTheDocument();
	});

	it('should disable the browser item when the tunnel is stopped', async () => {
		renderMenu();

		await openMenu();

		expect(
			screen.getByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		).toBeDisabled();
	});

	it('should open the local forward in the browser', async () => {
		useDaemonStore.setState({ settings: buildSettings() });
		const { gateway } = renderMenu({ state: { kind: 'active' } });
		await openMenu();

		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		);

		expect(gateway.openInBrowser).toHaveBeenCalledWith('127.0.0.1', 3333);
	});

	it('should open the details of the tunnel', async () => {
		renderMenu();
		await openMenu();

		await userEvent.click(screen.getByRole('menuitem', { name: 'Détails' }));

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelDetails',
			tunnelId: 'tunnel-1',
		});
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

	describe('when the tunnel is external', () => {
		const external = {
			state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
		} as const;

		it('should offer adopt, stop the process and ignore before the editing items', async () => {
			renderMenu(external);

			await openMenu();

			expect(
				screen.getAllByRole('menuitem').map((item) => item.textContent)
			).toEqual([
				'Détails',
				'Ouvrir dans le navigateur',
				'Adopter',
				'Arrêter le processus externe…',
				'Ignorer',
				'Modifier',
				'Redémarrer',
				'Supprimer',
			]);
		});

		it('should adopt the tunnel and confirm with a toast', async () => {
			const { gateway } = renderMenu(external);
			gateway.adoptExternal.mockResolvedValue(['tunnel-1']);
			await openMenu();

			await userEvent.click(screen.getByRole('menuitem', { name: 'Adopter' }));

			expect(gateway.adoptExternal).toHaveBeenCalledWith('tunnel-1');
			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toBe(
					'Tunnel Adonis adopté'
				)
			);
		});

		it('should announce the count when several tunnels are adopted', async () => {
			const { gateway } = renderMenu(external);
			gateway.adoptExternal.mockResolvedValue(['a', 'b']);
			await openMenu();

			await userEvent.click(screen.getByRole('menuitem', { name: 'Adopter' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toBe(
					'2 tunnels adoptés'
				)
			);
		});

		it('should push an error toast when the adoption fails', async () => {
			const gateway = createFakeGateway();
			gateway.adoptExternal.mockRejectedValue(new Error('boom'));
			renderMenu(external, gateway);
			await openMenu();

			await userEvent.click(screen.getByRole('menuitem', { name: 'Adopter' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toContain(
					'Adoption impossible'
				)
			);
		});

		it('should ask for confirmation before stopping the process', async () => {
			const { gateway } = renderMenu(external);
			await openMenu();

			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Arrêter le processus externe…' })
			);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Arrêter ssh (pid 4242) ?',
				})
			).toBeInTheDocument();
			expect(gateway.killExternal).not.toHaveBeenCalled();
		});

		it('should kill the process when confirmed', async () => {
			const { gateway } = renderMenu(external);
			await openMenu();
			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Arrêter le processus externe…' })
			);

			await userEvent.click(
				await screen.findByRole('button', { name: 'Arrêter le processus' })
			);

			expect(gateway.killExternal).toHaveBeenCalledWith(4242);
		});

		it('should not kill the process when cancelled', async () => {
			const { gateway } = renderMenu(external);
			await openMenu();
			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Arrêter le processus externe…' })
			);

			await userEvent.click(
				await screen.findByRole('button', { name: 'Annuler' })
			);

			expect(gateway.killExternal).not.toHaveBeenCalled();
		});

		it('should push an error toast when stopping the process fails', async () => {
			const gateway = createFakeGateway();
			gateway.killExternal.mockRejectedValue(new Error('boom'));
			renderMenu(external, gateway);
			await openMenu();
			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Arrêter le processus externe…' })
			);

			await userEvent.click(
				await screen.findByRole('button', { name: 'Arrêter le processus' })
			);

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toContain(
					'Arrêt du processus impossible'
				)
			);
		});

		it('should ignore the process', async () => {
			const { gateway } = renderMenu(external);
			await openMenu();

			await userEvent.click(screen.getByRole('menuitem', { name: 'Ignorer' }));

			expect(gateway.ignoreExternal).toHaveBeenCalledWith(4242);
		});

		it('should have no accessibility violation when open', async () => {
			const { container } = renderMenu(external);
			await openMenu();

			expect(await axe(container)).toHaveNoViolations();
		});
	});

	describe('when the tunnel is in conflict', () => {
		it('should offer to stop the process holding the port', async () => {
			renderMenu({
				state: { kind: 'conflict', owner: { pid: 99, process: 'node.exe' } },
			});

			await openMenu();

			expect(
				screen.getAllByRole('menuitem').map((item) => item.textContent)
			).toEqual([
				'Détails',
				'Ouvrir dans le navigateur',
				'Arrêter le processus…',
				'Modifier',
				'Redémarrer',
				'Supprimer',
			]);
		});

		it('should kill the owner when confirmed', async () => {
			const { gateway } = renderMenu({
				state: { kind: 'conflict', owner: { pid: 99, process: 'node.exe' } },
			});
			await openMenu();
			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Arrêter le processus…' })
			);

			await userEvent.click(
				await screen.findByRole('button', { name: 'Arrêter le processus' })
			);

			expect(gateway.killExternal).toHaveBeenCalledWith(99);
		});

		it('should not offer to stop a process when the owner is unknown', async () => {
			renderMenu({ state: { kind: 'conflict' } });

			await openMenu();

			expect(
				screen.getAllByRole('menuitem').map((item) => item.textContent)
			).toEqual([
				'Détails',
				'Ouvrir dans le navigateur',
				'Modifier',
				'Redémarrer',
				'Supprimer',
			]);
		});
	});
});
