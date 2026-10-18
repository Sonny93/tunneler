import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';

const ACTIVE = { kind: 'active' } as const;

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'a', name: 'Alpha', host: 'nova', state: ACTIVE }),
			buildTunnel({ id: 'b', name: 'Beta', host: 'nova', tags: ['dev'] }),
			buildTunnel({
				id: 'c',
				name: 'Gamma',
				host: 'nova',
				tags: ['dev', 'db'],
			}),
			buildTunnel({ id: 'd', name: 'Delta', host: 'zeta' }),
		],
	});
}

function renderList(gateway = createFakeGateway()) {
	seedTunnels();
	return renderWithGateway(
		<>
			<TunnelList />
			<ModalProvider />
		</>,
		gateway
	);
}

const rowCheckbox = (name: string) =>
	screen.getByRole('checkbox', { name: `Sélectionner ${name}` });
const selectAll = () =>
	screen.getByRole('checkbox', { name: 'Tout sélectionner' });
const toolbar = () =>
	screen.getByRole('toolbar', { name: 'Actions sur la sélection' });
const toolbarButton = (name: string) =>
	within(toolbar()).getByRole('button', { name });
const lastToastMessage = () => useToastStore.getState().toasts.at(-1)?.message;

async function select(...names: string[]) {
	for (const name of names) await userEvent.click(rowCheckbox(name));
}

describe('TunnelList selection', () => {
	it('should show no toolbar while nothing is selected', () => {
		renderList();

		expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
	});

	it('should select a tunnel with its checkbox', async () => {
		renderList();

		await select('Alpha');

		expect(rowCheckbox('Alpha')).toBeChecked();
		expect(rowCheckbox('Beta')).not.toBeChecked();
	});

	it('should count the selection in the toolbar with a singular', async () => {
		renderList();

		await select('Alpha');

		expect(toolbar()).toHaveTextContent('1 tunnel sélectionné');
	});

	it('should count the selection in the toolbar with a plural', async () => {
		renderList();

		await select('Alpha', 'Beta');

		expect(toolbar()).toHaveTextContent('2 tunnels sélectionnés');
	});

	it('should select the range from the last toggled row with Shift+click', async () => {
		const user = userEvent.setup();
		renderList();
		await select('Alpha');

		await user.keyboard('{Shift>}');
		await user.click(rowCheckbox('Gamma'));
		await user.keyboard('{/Shift}');

		expect(
			['Alpha', 'Beta', 'Gamma', 'Delta'].map(
				(name) => (rowCheckbox(name) as HTMLInputElement).checked
			)
		).toEqual([true, true, true, false]);
	});

	it('should toggle a checkbox with Space', async () => {
		renderList();
		rowCheckbox('Beta').focus();

		await userEvent.keyboard(' ');

		expect(rowCheckbox('Beta')).toBeChecked();
	});

	it('should select every visible tunnel with the select-all checkbox', async () => {
		renderList();

		await userEvent.click(selectAll());

		expect(toolbar()).toHaveTextContent('4 tunnels sélectionnés');
		expect(selectAll()).toBeChecked();
	});

	it('should mark the select-all checkbox as mixed while the selection is partial', async () => {
		renderList();

		await select('Alpha');

		expect(selectAll()).toBePartiallyChecked();
	});

	it('should select everything from a partial selection', async () => {
		renderList();
		await select('Alpha');

		await userEvent.click(selectAll());

		expect(toolbar()).toHaveTextContent('4 tunnels sélectionnés');
	});

	it('should clear the selection with the select-all checkbox when everything is selected', async () => {
		renderList();
		await userEvent.click(selectAll());

		await userEvent.click(selectAll());

		expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
	});

	it('should only select the visible tunnels', async () => {
		renderList();
		act(() => useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' }));

		await userEvent.click(selectAll());

		expect(toolbar()).toHaveTextContent('2 tunnels sélectionnés');
	});

	it('should select every visible tunnel with Ctrl+A inside the list', async () => {
		renderList();
		rowCheckbox('Beta').focus();

		await userEvent.keyboard('{Control>}a{/Control}');

		expect(toolbar()).toHaveTextContent('4 tunnels sélectionnés');
	});

	it('should clear the selection with Escape inside the list', async () => {
		renderList();
		await select('Alpha');
		rowCheckbox('Alpha').focus();

		await userEvent.keyboard('{Escape}');

		expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
	});

	it('should keep the selection when Escape closes an open menu', async () => {
		renderList();
		await select('Alpha');
		await userEvent.click(
			screen.getByRole('button', { name: 'Actions pour Beta' })
		);

		await userEvent.keyboard('{Escape}');

		expect(toolbar()).toHaveTextContent('1 tunnel sélectionné');
	});

	it('should drop from the selection the tunnels a filter hides', async () => {
		renderList();
		await select('Alpha', 'Beta');

		act(() => useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' }));

		expect(toolbar()).toHaveTextContent('1 tunnel sélectionné');
		expect(useUiStore.getState().selectedTunnelIds.has('a')).toBe(false);
	});

	it('should drop from the selection the tunnels that are deleted', async () => {
		renderList();
		await select('Alpha', 'Beta');

		act(() =>
			useDaemonStore.setState({
				tunnels: useDaemonStore
					.getState()
					.tunnels.filter(({ id }) => id !== 'a'),
			})
		);

		expect(toolbar()).toHaveTextContent('1 tunnel sélectionné');
	});

	it('should announce the new count politely', async () => {
		renderList();

		await select('Alpha', 'Beta');

		expect(useAnnouncerStore.getState().polite.message).toBe(
			'2 tunnels sélectionnés'
		);
	});

	it('should announce nothing before the selection changes', () => {
		renderList();

		expect(useAnnouncerStore.getState().polite.message).toBe('');
	});

	it('should clear the selection with the deselect button', async () => {
		renderList();
		await select('Alpha');

		await userEvent.click(toolbarButton('Tout désélectionner'));

		expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
	});

	it('should have no axe violations with a selection', async () => {
		const { container } = renderList();
		await select('Alpha', 'Beta');

		expect(await axe(container)).toHaveNoViolations();
	});
});

describe('TunnelList toolbar keyboard', () => {
	it('should make the toolbar a single tab stop', async () => {
		renderList();
		await select('Alpha');

		expect(
			within(toolbar())
				.getAllByRole('button')
				.filter((button) => button.getAttribute('tabindex') === '0')
		).toHaveLength(1);
	});

	it('should move between toolbar buttons with the arrow keys', async () => {
		renderList();
		await select('Alpha', 'Beta');
		toolbarButton('Arrêter').focus();

		await userEvent.keyboard('{ArrowRight}');

		expect(toolbarButton('Redémarrer')).toHaveFocus();
	});

	it('should skip disabled buttons', async () => {
		renderList();
		await select('Beta');
		toolbarButton('Démarrer').focus();

		await userEvent.keyboard('{ArrowRight}');

		expect(toolbarButton('Tags…')).toHaveFocus();
	});

	it('should jump to the last button with End', async () => {
		renderList();
		await select('Alpha');
		toolbarButton('Arrêter').focus();

		await userEvent.keyboard('{End}');

		expect(toolbarButton('Tout désélectionner')).toHaveFocus();
	});
});

describe('TunnelList bulk actions', () => {
	it('should start only the tunnels that are not running', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta', 'Gamma');

		await userEvent.click(toolbarButton('Démarrer'));

		await waitFor(() => expect(gateway.startTunnel).toHaveBeenCalledTimes(2));
		expect(gateway.startTunnel).toHaveBeenCalledWith('b');
		expect(gateway.startTunnel).toHaveBeenCalledWith('c');
		expect(lastToastMessage()).toBe('2 tunnels démarrés');
	});

	it('should stop only the running tunnels', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta');

		await userEvent.click(toolbarButton('Arrêter'));

		await waitFor(() => expect(gateway.stopTunnel).toHaveBeenCalledTimes(1));
		expect(gateway.stopTunnel).toHaveBeenCalledWith('a');
		expect(lastToastMessage()).toBe('1 tunnel arrêté');
	});

	it('should restart only the running tunnels', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta');

		await userEvent.click(toolbarButton('Redémarrer'));

		await waitFor(() => expect(gateway.restartTunnel).toHaveBeenCalledTimes(1));
		expect(gateway.restartTunnel).toHaveBeenCalledWith('a');
	});

	it('should disable stopping when no selected tunnel is running', async () => {
		renderList();

		await select('Beta');

		expect(toolbarButton('Arrêter')).toBeDisabled();
	});

	it('should report a partial failure in one error toast', async () => {
		const gateway = createFakeGateway();
		gateway.startTunnel.mockImplementation((id) =>
			id === 'c' ? Promise.reject(new Error('port occupé')) : Promise.resolve()
		);
		renderList(gateway);
		await select('Beta', 'Gamma', 'Delta');

		await userEvent.click(toolbarButton('Démarrer'));

		await waitFor(() =>
			expect(lastToastMessage()).toBe(
				'2 sur 3 démarrés ; échec : Gamma (port occupé)'
			)
		);
		expect(useToastStore.getState().toasts.at(-1)?.tone).toBe('error');
	});

	it('should ask for confirmation, listing the names, before deleting', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta');

		await userEvent.click(toolbarButton('Supprimer'));

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Supprimer 2 tunnels ?',
		});
		expect(dialog).toHaveTextContent('Alpha');
		expect(dialog).toHaveTextContent('Beta');
		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
	});

	it('should keep everything when the deletion is cancelled', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta');
		await userEvent.click(toolbarButton('Supprimer'));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Annuler' })
		);

		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
		expect(toolbar()).toHaveTextContent('2 tunnels sélectionnés');
	});

	it('should delete the selection, clear it and focus a remaining row', async () => {
		const gateway = createFakeGateway();
		gateway.deleteTunnel.mockImplementation((id) => {
			useDaemonStore.setState({
				tunnels: useDaemonStore.getState().tunnels.filter((t) => t.id !== id),
			});
			return Promise.resolve();
		});
		renderList(gateway);
		await select('Beta', 'Gamma');
		await userEvent.click(toolbarButton('Supprimer'));

		await userEvent.click(
			within(await screen.findByRole('alertdialog')).getByRole('button', {
				name: 'Supprimer',
			})
		);

		await waitFor(() =>
			expect(screen.queryByRole('toolbar')).not.toBeInTheDocument()
		);
		expect(lastToastMessage()).toBe('2 tunnels supprimés');
		expect(
			screen.getByRole('switch', { name: /^Tunnel Delta,/ })
		).toHaveFocus();
	});
});

describe('TunnelList bulk tags', () => {
	it('should add a tag to the tunnels that lack it', async () => {
		const { gateway } = renderList();
		await select('Beta', 'Gamma', 'Delta');
		await userEvent.click(toolbarButton('Tags…'));
		const dialog = await screen.findByRole('dialog', {
			name: 'Tags de 3 tunnels',
		});

		await userEvent.type(within(dialog).getByLabelText('Tag à ajouter'), 'dev');
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Appliquer' })
		);

		await waitFor(() => expect(gateway.updateTunnel).toHaveBeenCalledTimes(1));
		expect(gateway.updateTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'd', tags: ['dev'] })
		);
		expect(lastToastMessage()).toBe('1 tunnel mis à jour');
	});

	it('should require a tag when adding', async () => {
		const { gateway } = renderList();
		await select('Beta');
		await userEvent.click(toolbarButton('Tags…'));
		const dialog = await screen.findByRole('dialog');

		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Appliquer' })
		);

		expect(within(dialog).getByLabelText('Tag à ajouter')).toBeInvalid();
		expect(within(dialog).getByText('Saisissez un tag.')).toBeInTheDocument();
		expect(gateway.updateTunnel).not.toHaveBeenCalled();
	});

	it('should remove a tag present in the selection', async () => {
		const { gateway } = renderList();
		await select('Beta', 'Gamma');
		await userEvent.click(toolbarButton('Tags…'));
		const dialog = await screen.findByRole('dialog');

		await userEvent.click(
			within(dialog).getByRole('radio', { name: 'Retirer un tag' })
		);
		await userEvent.selectOptions(
			within(dialog).getByLabelText('Tag à retirer'),
			'dev'
		);
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Appliquer' })
		);

		await waitFor(() => expect(gateway.updateTunnel).toHaveBeenCalledTimes(2));
		expect(gateway.updateTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'c', tags: ['db'] })
		);
	});

	it('should not update anything when cancelled', async () => {
		const { gateway } = renderList();
		await select('Beta');
		await userEvent.click(toolbarButton('Tags…'));
		const dialog = await screen.findByRole('dialog');

		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Annuler' })
		);

		expect(gateway.updateTunnel).not.toHaveBeenCalled();
	});

	it('should suggest the existing tags', async () => {
		renderList();
		await select('Delta');
		await userEvent.click(toolbarButton('Tags…'));
		const dialog = await screen.findByRole('dialog');

		const input = within(dialog).getByLabelText('Tag à ajouter');
		const options = document.querySelectorAll(
			`datalist[id="${input.getAttribute('list')}"] option`
		);

		expect([...options].map((option) => option.getAttribute('value'))).toEqual([
			'db',
			'dev',
		]);
	});
});

describe('TunnelList context menu', () => {
	const rightClick = (name: string) =>
		userEvent.pointer({
			keys: '[MouseRight]',
			target: screen.getByText(name),
		});

	it('should open the tunnel menu on right-click', async () => {
		renderList();

		await rightClick('Alpha');

		const menu = await screen.findByRole('menu', {
			name: 'Actions pour Alpha',
		});
		expect(
			within(menu)
				.getAllByRole('menuitem')
				.map((item) => item.textContent)
		).toContain('Détails');
	});

	it('should open the selection menu on a row of a multi-selection', async () => {
		renderList();
		await select('Alpha', 'Beta');

		await rightClick('Beta');

		const menu = await screen.findByRole('menu', { name: 'Actions pour Beta' });
		expect(
			within(menu)
				.getAllByRole('menuitem')
				.map((item) => item.textContent)
		).toEqual(['Démarrer', 'Arrêter', 'Redémarrer', 'Tags…', 'Supprimer']);
		expect(
			within(menu).getByRole('group', { name: '2 tunnels sélectionnés' })
		).toBeInTheDocument();
	});

	it('should apply a selection menu action to the selection', async () => {
		const { gateway } = renderList();
		await select('Alpha', 'Beta');
		await rightClick('Beta');

		await userEvent.click(
			await screen.findByRole('menuitem', { name: 'Démarrer' })
		);

		await waitFor(() => expect(gateway.startTunnel).toHaveBeenCalledWith('b'));
		expect(gateway.startTunnel).toHaveBeenCalledTimes(1);
	});

	it('should open the tunnel menu on a row outside the selection', async () => {
		renderList();
		await select('Alpha', 'Beta');

		await rightClick('Gamma');

		expect(
			await screen.findByRole('menuitem', { name: 'Détails' })
		).toBeInTheDocument();
	});

	it('should open the tunnel menu when only one tunnel is selected', async () => {
		renderList();
		await select('Alpha');

		await rightClick('Alpha');

		expect(
			await screen.findByRole('menuitem', { name: 'Détails' })
		).toBeInTheDocument();
	});

	it('should have no axe violations with the context menu open', async () => {
		renderList();
		await rightClick('Alpha');
		await screen.findByRole('menu');

		expect(
			await axe(document.body, { rules: { region: { enabled: false } } })
		).toHaveNoViolations();
	});
});
