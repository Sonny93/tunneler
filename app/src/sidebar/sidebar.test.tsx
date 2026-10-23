import { ModalProvider } from '@minimalstuff/ui';
import { act, fireEvent, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { useToastStore } from '#/shared/toasts/toast_store';
import { Sidebar } from '#/sidebar/sidebar';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
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
			screen.getByRole('navigation', { name: 'Tunnel filters' })
		).toBeInTheDocument();
		const allFilter = screen.getByRole('button', { name: /All tunnels/ });
		expect(allFilter).toHaveTextContent('2/3 active');
		expect(allFilter).toHaveAccessibleName(/^All tunnels\s?2 of 3 active$/);
		expect(allFilter).toHaveAttribute('aria-current', 'true');
		expect(
			screen.getAllByRole('heading').map((heading) => heading.textContent)
		).toEqual(['Hosts', 'Tags']);
		expect(screen.getByRole('button', { name: /^nova/ })).toHaveTextContent(
			'1/2 active'
		);
		const tagList = within(
			screen.getByRole('heading', { name: 'Tags' }).parentElement as HTMLElement
		);
		expect(
			tagList.getAllByRole('button').map((button) => button.textContent)
		).toEqual(['Db0/1 active0 of 1 active', 'dev1/2 active1 of 2 active']);
		expect(
			screen.getByRole('switch', { name: 'All tunnels of tag dev' })
		).toHaveAccessibleDescription('1 of 2 active');
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

		await userEvent.click(screen.getByRole('button', { name: /All tunnels/ }));

		expect(useUiStore.getState().filter).toEqual({ kind: 'all' });
	});

	it('should mark all as current when the filtered tag disappeared', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'tag', tag: 'gone' } });
		renderWithGateway(<Sidebar />);

		expect(screen.getByRole('button', { name: /All tunnels/ })).toHaveAttribute(
			'aria-current',
			'true'
		);
	});

	it('should start the whole tag when not every tunnel runs', async () => {
		seedTunnels();
		const { gateway } = renderWithGateway(<Sidebar />);

		await userEvent.click(
			screen.getByRole('switch', { name: 'All tunnels of tag dev' })
		);

		expect(gateway.startTag).toHaveBeenCalledWith('dev');
	});

	it('should stop the whole tag when every tunnel runs', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ tags: ['dev'], state: ACTIVE })],
		});
		const { gateway } = renderWithGateway(<Sidebar />);
		const tagSwitch = screen.getByRole('switch', {
			name: 'All tunnels of tag dev',
		});

		await userEvent.click(tagSwitch);

		expect(tagSwitch).toBeChecked();
		expect(gateway.stopTag).toHaveBeenCalledWith('dev');
	});

	it('should show the tag switch on and stop the tag when every tunnel is in standby', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
				buildTunnel({ id: 'b', tags: ['dev'], state: { kind: 'standby' } }),
			],
		});
		const { gateway } = renderWithGateway(<Sidebar />);
		const tagSwitch = screen.getByRole('switch', {
			name: 'All tunnels of tag dev',
		});

		await userEvent.click(tagSwitch);

		expect(tagSwitch).toBeChecked();
		expect(gateway.stopTag).toHaveBeenCalledWith('dev');
		expect(gateway.startTag).not.toHaveBeenCalled();
	});

	it('should show the tag switch off and start the tag when standby tunnels are mixed with stopped ones', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
				buildTunnel({ id: 'b', tags: ['dev'] }),
			],
		});
		const { gateway } = renderWithGateway(<Sidebar />);
		const tagSwitch = screen.getByRole('switch', {
			name: 'All tunnels of tag dev',
		});

		await userEvent.click(tagSwitch);

		expect(tagSwitch).not.toBeChecked();
		expect(gateway.startTag).toHaveBeenCalledWith('dev');
		expect(gateway.stopTag).not.toHaveBeenCalled();
	});

	it('should push an error toast when starting a tag fails', async () => {
		seedTunnels();
		const gateway = createFakeGateway();
		gateway.startTag.mockRejectedValue(new Error('boom'));
		renderWithGateway(<Sidebar />, gateway);

		await userEvent.click(
			screen.getByRole('switch', { name: 'All tunnels of tag dev' })
		);

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			"Couldn't start the tunnels"
		);
	});

	describe('all tunnels context menu', () => {
		const openAllMenu = (gateway = createFakeGateway()) => {
			const rendered = renderWithGateway(<Sidebar />, gateway);
			fireEvent.contextMenu(
				screen.getByRole('button', { name: /All tunnels/ })
			);
			return rendered;
		};

		it('should send every awake tunnel to standby from the menu', async () => {
			useDaemonStore.setState({ tunnels: [buildTunnel({ isAwake: true })] });
			const { gateway } = openAllMenu();

			await userEvent.click(
				screen.getByRole('menuitem', {
					name: 'Send awake tunnels to standby',
				})
			);

			expect(gateway.standbyAll).toHaveBeenCalledTimes(1);
		});

		it('should disable the standby item when no tunnel is awake and have no axe violations', async () => {
			seedTunnels();
			openAllMenu();

			expect(
				screen.getByRole('menuitem', {
					name: 'Send awake tunnels to standby',
				})
			).toBeDisabled();
			expect(
				await axe(document.body, { rules: { region: { enabled: false } } })
			).toHaveNoViolations();
		});

		it('should name the menu and keep focusing the all entry on request', () => {
			seedTunnels();
			renderWithGateway(<Sidebar />);

			act(() => useUiStore.setState({ isAllFilterFocusPending: true }));

			expect(screen.getByRole('button', { name: /All tunnels/ })).toHaveFocus();
		});
	});

	describe('tag context menu', () => {
		it('should send the awake tunnels of the tag to standby from the menu', async () => {
			useDaemonStore.setState({
				tunnels: [buildTunnel({ tags: ['dev'], isAwake: true })],
			});
			const { gateway } = renderWithGateway(<Sidebar />);
			fireEvent.contextMenu(screen.getByRole('button', { name: /^dev/ }));

			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Send the tag to standby' })
			);

			expect(gateway.standbyTag).toHaveBeenCalledWith('dev');
		});

		it('should disable the standby item of a tag without awake tunnel', () => {
			useDaemonStore.setState({ tunnels: [buildTunnel({ tags: ['dev'] })] });
			renderWithGateway(<Sidebar />);

			fireEvent.contextMenu(screen.getByRole('button', { name: /^dev/ }));

			expect(
				screen.getByRole('menuitem', { name: 'Send the tag to standby' })
			).toBeDisabled();
		});

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
			).toEqual([
				'Rename…',
				'Delete tag…',
				'Start the tag',
				'Send the tag to standby',
			]);
		});

		it('should offer to stop a fully running tag', () => {
			useDaemonStore.setState({
				tunnels: [buildTunnel({ tags: ['dev'], state: ACTIVE })],
			});
			renderWithGateway(<Sidebar />);

			fireEvent.contextMenu(screen.getByRole('button', { name: /^dev/ }));

			expect(
				screen.getByRole('menuitem', { name: 'Stop the tag' })
			).toBeInTheDocument();
		});

		it('should start the tag from the menu', async () => {
			const user = userEvent.setup();
			const { gateway } = openTagMenu(/^dev/);

			await user.click(screen.getByRole('menuitem', { name: 'Start the tag' }));

			expect(gateway.startTag).toHaveBeenCalledWith('dev');
		});

		it('should open the rename dialog from the menu', async () => {
			const user = userEvent.setup();
			openTagMenu(/^dev/);

			await user.click(screen.getByRole('menuitem', { name: 'Rename…' }));

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

			await user.click(screen.getByRole('menuitem', { name: 'Delete tag…' }));

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Delete tag dev?',
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

		expect(screen.getByRole('button', { name: /All tunnels/ })).toHaveFocus();
		expect(useUiStore.getState().isAllFilterFocusPending).toBe(false);
	});
});
