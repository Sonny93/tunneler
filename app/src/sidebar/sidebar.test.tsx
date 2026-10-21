import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, fireEvent, screen, within } from '@testing-library/react';

import { Sidebar } from '#/sidebar/sidebar';
import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';

const ACTIVE = { kind: 'active' } as const;

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'a', host: 'nova', tags: ['dev'], state: ACTIVE }),
			buildTunnel({ id: 'b', host: 'nova', tags: ['dev', 'Db'] }),
			buildTunnel({ id: 'c', host: 'zeta', tags: [], state: ACTIVE }),
		],
	});
}

describe('Sidebar', () => {
	it('should list the hosts then the tags with their active counts in a labelled navigation', async () => {
		seedTunnels();
		const { container } = renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('navigation', { name: 'Filtres des tunnels' })
		).toBeInTheDocument();
		const allFilter = screen.getByRole('button', { name: /Tous les tunnels/ });
		expect(allFilter).toHaveTextContent('2/3 actifs');
		expect(allFilter).toHaveAccessibleName(
			/^Tous les tunnels\s?2 sur 3 actifs$/
		);
		expect(allFilter).toHaveAttribute('aria-current', 'true');
		expect(
			screen.getAllByRole('heading').map((heading) => heading.textContent)
		).toEqual(['Hosts', 'Tags']);
		expect(screen.getByRole('button', { name: /^nova/ })).toHaveTextContent(
			'1/2 actifs'
		);
		const tagList = within(
			screen.getByRole('heading', { name: 'Tags' }).parentElement as HTMLElement
		);
		expect(
			tagList.getAllByRole('button').map((button) => button.textContent)
		).toEqual(['Db0/1 actifs0 sur 1 actifs', 'dev1/2 actifs1 sur 2 actifs']);
		expect(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		).toHaveAccessibleDescription('1 sur 2 actifs');
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should hide the tags section when no tunnel has a tag', () => {
		useDaemonStore.setState({ tunnels: [buildTunnel()] });
		renderWithGateway(<Sidebar />);

		expect(
			screen.queryByRole('heading', { name: 'Tags' })
		).not.toBeInTheDocument();
	});

	it('should filter by tag when its button is clicked', async () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		await userEvent.click(screen.getByRole('button', { name: /^dev/ }));

		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'dev' });
		expect(screen.getByRole('button', { name: /^dev/ })).toHaveAttribute(
			'aria-current',
			'true'
		);
	});

	it('should filter by host when its button is clicked', async () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		await userEvent.click(screen.getByRole('button', { name: /^zeta/ }));

		expect(useUiStore.getState().filter).toEqual({
			kind: 'host',
			host: 'zeta',
		});
	});

	it('should go back to all tunnels from the all filter', async () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'host', host: 'zeta' } });
		renderWithGateway(<Sidebar />);

		await userEvent.click(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		);

		expect(useUiStore.getState().filter).toEqual({ kind: 'all' });
	});

	it('should mark all as current when the filtered tag disappeared', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'tag', tag: 'gone' } });
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		).toHaveAttribute('aria-current', 'true');
	});

	it('should start the whole tag when not every tunnel runs', async () => {
		seedTunnels();
		const { gateway } = renderWithGateway(<Sidebar />);

		await userEvent.click(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		);

		expect(gateway.startTag).toHaveBeenCalledWith('dev');
	});

	it('should stop the whole tag when every tunnel runs', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ tags: ['dev'], state: ACTIVE })],
		});
		const { gateway } = renderWithGateway(<Sidebar />);
		const tagSwitch = screen.getByRole('switch', {
			name: 'Tous les tunnels du tag dev',
		});

		await userEvent.click(tagSwitch);

		expect(tagSwitch).toBeChecked();
		expect(gateway.stopTag).toHaveBeenCalledWith('dev');
	});

	it('should push an error toast when starting a tag fails', async () => {
		seedTunnels();
		const gateway = createFakeGateway();
		gateway.startTag.mockRejectedValue(new Error('boom'));
		renderWithGateway(<Sidebar />, gateway);

		await userEvent.click(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		);

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			'Démarrage impossible'
		);
	});

	describe('tag context menu', () => {
		const openTagMenu = (name: RegExp) => {
			seedTunnels();
			const rendered = renderWithGateway(<Sidebar />);
			fireEvent.contextMenu(screen.getByRole('button', { name }));
			return rendered;
		};

		it('should offer to rename and start a partially running tag', async () => {
			openTagMenu(/^dev/);

			expect(
				screen.getAllByRole('menuitem').map((item) => item.textContent)
			).toEqual(['Renommer…', 'Supprimer le tag…', 'Démarrer le tag']);
		});

		it('should offer to stop a fully running tag', () => {
			useDaemonStore.setState({
				tunnels: [buildTunnel({ tags: ['dev'], state: ACTIVE })],
			});
			renderWithGateway(<Sidebar />);

			fireEvent.contextMenu(screen.getByRole('button', { name: /^dev/ }));

			expect(
				screen.getByRole('menuitem', { name: 'Arrêter le tag' })
			).toBeInTheDocument();
		});

		it('should start the tag from the menu', async () => {
			const user = userEvent.setup();
			const { gateway } = openTagMenu(/^dev/);

			await user.click(
				screen.getByRole('menuitem', { name: 'Démarrer le tag' })
			);

			expect(gateway.startTag).toHaveBeenCalledWith('dev');
		});

		it('should open the rename dialog from the menu', async () => {
			const user = userEvent.setup();
			openTagMenu(/^dev/);

			await user.click(screen.getByRole('menuitem', { name: 'Renommer…' }));

			expect(useUiStore.getState().tagRename).toEqual({
				tag: 'dev',
				origin: 'sidebar',
			});
		});

		it('should ask for confirmation before deleting the tag from the menu', async () => {
			const user = userEvent.setup();
			seedTunnels();
			const { gateway } = renderWithGateway(
				<>
					<ModalProvider />
					<Sidebar />
				</>
			);
			fireEvent.contextMenu(screen.getByRole('button', { name: /^dev/ }));

			await user.click(
				screen.getByRole('menuitem', { name: 'Supprimer le tag…' })
			);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Supprimer le tag dev ?',
				})
			).toBeInTheDocument();
			expect(gateway.deleteTag).not.toHaveBeenCalled();
		});

		it('should focus the row of a just-renamed tag', () => {
			seedTunnels();
			renderWithGateway(<Sidebar />);

			act(() => useUiStore.getState().focusTagRow('dev'));

			expect(screen.getByRole('button', { name: /^dev/ })).toHaveFocus();
			expect(useUiStore.getState().pendingFocusTag).toBeNull();
		});
	});

	it('should focus the all tunnels filter when asked', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		act(() => useUiStore.getState().focusAllFilter());

		expect(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		).toHaveFocus();
		expect(useUiStore.getState().isAllFilterFocusPending).toBe(false);
	});
});
