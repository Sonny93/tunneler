import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelActionsMenu } from '#/tunnels/list/tunnel_actions_menu';
import { buildOpener, buildSettings, buildTunnel } from '#/testing/factories';

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
	await userEvent.click(screen.getByRole('button', { name: /^Actions pour / }));
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

	it('should name the group of control items', async () => {
		renderMenu({ state: { kind: 'active' } });

		await openMenu();

		const group = screen.getByRole('group', { name: 'Contrôle' });
		expect(
			within(group)
				.getAllByRole('menuitem')
				.map((item) => item.textContent)
		).toEqual(['Modifier', 'Redémarrer', 'Supprimer']);
	});

	it('should offer to put an awake tunnel back to standby after the restart', async () => {
		renderMenu({ state: { kind: 'active' }, isAwake: true });

		await openMenu();

		const group = screen.getByRole('group', { name: 'Contrôle' });
		expect(
			within(group)
				.getAllByRole('menuitem')
				.map((item) => item.textContent)
		).toEqual(['Modifier', 'Redémarrer', 'Mettre en veille', 'Supprimer']);
	});

	it('should send the awake tunnel to standby from the menu', async () => {
		const { gateway } = renderMenu({
			state: { kind: 'active' },
			isAwake: true,
		});
		await openMenu();

		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Mettre en veille' })
		);

		await waitFor(() =>
			expect(gateway.standbyTunnel).toHaveBeenCalledWith('tunnel-1')
		);
	});

	it('should not offer standby when the tunnel is not awake', async () => {
		renderMenu({ state: { kind: 'active' } });

		await openMenu();

		expect(
			screen.queryByRole('menuitem', { name: 'Mettre en veille' })
		).not.toBeInTheDocument();
	});

	it('should not offer the browser for a remote forward', async () => {
		renderMenu({ kind: 'remote' });

		await openMenu();

		expect(
			screen.queryByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		).not.toBeInTheDocument();
	});

	it('should keep the browser item enabled when the tunnel is stopped', async () => {
		renderMenu();

		await openMenu();

		expect(
			screen.getByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		).toBeEnabled();
	});

	it('should ask the daemon to open the local forward', async () => {
		useDaemonStore.setState({ settings: buildSettings() });
		const { gateway } = renderMenu({ state: { kind: 'active' } });
		await openMenu();

		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Ouvrir dans le navigateur' })
		);

		expect(gateway.openTunnel).toHaveBeenCalledWith('tunnel-1', undefined);
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
				name: 'Supprimer le tunnel 3333 → nova:3333 ?',
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
			'Tunnel 3333 → nova:3333 supprimé'
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
					'Tunnel 3333 → nova:3333 adopté'
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

	describe('with applications', () => {
		const vnc = buildOpener();
		const appTunnel = {
			defaultOpener: { id: 'remmina-vnc', name: 'Remmina (VNC)' },
		};

		it('should word the primary item after the default application, list the browser in a labelled group and have no axe violations', async () => {
			useDaemonStore.setState({ openers: [vnc] });
			renderMenu(appTunnel);

			await openMenu();

			expect(
				screen.getByRole('menuitem', { name: 'Ouvrir avec Remmina (VNC)' })
			).toBeInTheDocument();
			const group = screen.getByRole('group', { name: 'Ouvrir avec' });
			expect(
				within(group).getByRole('menuitem', { name: 'Navigateur' })
			).toBeInTheDocument();
			expect(await axe(screen.getByRole('menu'))).toHaveNoViolations();
		});

		it('should open with the chosen alternative', async () => {
			useDaemonStore.setState({ openers: [vnc] });
			const { gateway } = renderMenu(appTunnel);
			await openMenu();

			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Navigateur' })
			);

			expect(gateway.openTunnel).toHaveBeenCalledWith('tunnel-1', 'browser');
		});

		it('should offer the applications as alternatives to the browser', async () => {
			useDaemonStore.setState({ openers: [vnc] });
			renderMenu();

			await openMenu();

			expect(
				within(screen.getByRole('group', { name: 'Ouvrir avec' })).getByRole(
					'menuitem',
					{ name: 'Remmina (VNC)' }
				)
			).toBeInTheDocument();
		});

		it('should show no group when there is no alternative', async () => {
			renderMenu({ defaultOpener: { id: 'browser', name: 'Browser' } });

			await openMenu();

			expect(
				screen.queryByRole('group', { name: 'Ouvrir avec' })
			).not.toBeInTheDocument();
		});
	});
});
