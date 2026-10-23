import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { Overlays } from '#/overlays';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

function renderPalette() {
	useDaemonStore.setState({
		settings: buildSettings(),
		tunnels: [buildTunnel()],
		hosts: ['nova'],
	});
	return renderWithGateway(
		<>
			<button>Trigger</button>
			<ModalProvider />
			<Overlays />
		</>,
		createFakeGateway()
	);
}

async function openPalette(user: ReturnType<typeof userEvent.setup>) {
	await user.keyboard('{Control>}k{/Control}');
	return screen.findByRole('dialog', { name: 'Command palette' });
}

const getInput = () =>
	screen.getByRole('combobox', { name: 'Search a command' });

describe('CommandPalette', () => {
	it('should render nothing while closed', () => {
		renderPalette();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should open with Ctrl+K, focus the search field and have no axe violations', async () => {
		const user = userEvent.setup();
		renderPalette();

		await openPalette(user);

		expect(getInput()).toHaveFocus();
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should open with Meta+K', async () => {
		const user = userEvent.setup();
		renderPalette();

		await user.keyboard('{Meta>}k{/Meta}');

		expect(
			await screen.findByRole('dialog', { name: 'Command palette' })
		).toBeInTheDocument();
	});

	it('should open from the store flag', async () => {
		renderPalette();

		useUiStore.getState().setPaletteOpen(true);

		expect(
			await screen.findByRole('dialog', { name: 'Command palette' })
		).toBeInTheDocument();
	});

	it('should not open over a drawer', async () => {
		const user = userEvent.setup();
		renderPalette();
		useUiStore.getState().openSettings();

		await user.keyboard('{Control>}k{/Control}');

		expect(useUiStore.getState().isPaletteOpen).toBe(false);
	});

	it('should wire the combobox to its listbox and active option', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		const listbox = screen.getByRole('listbox', { name: 'Commands' });
		const [firstOption] = screen.getAllByRole('option');
		expect(getInput()).toHaveAttribute('aria-expanded', 'true');
		expect(getInput()).toHaveAttribute('aria-controls', listbox.id);
		expect(getInput()).toHaveAttribute(
			'aria-activedescendant',
			firstOption?.id
		);
		expect(firstOption).toHaveAttribute('aria-selected', 'true');
	});

	it('should show the label and the category of each option', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		const option = screen.getByRole('option', {
			name: /New tunnel(?! temporary)/,
		});

		expect(option).toHaveTextContent('General');
	});

	it('should open the temporary tunnel form from the palette', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'temporary');
		await user.keyboard('{Enter}');

		expect(
			await screen.findByRole('dialog', { name: 'New temporary tunnel' })
		).toBeInTheDocument();
	});

	it('should open the documentation home from the palette', async () => {
		const user = userEvent.setup();
		const { gateway } = renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'documentation');
		await user.keyboard('{Enter}');

		await waitFor(() =>
			expect(gateway.openDocumentation).toHaveBeenCalledWith('home', 'en')
		);
	});

	it('should announce the number of commands', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), '3333');

		expect(screen.getByRole('status')).toHaveTextContent('4 commands');
	});

	it('should filter ignoring case and accents', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'SETTINGS: SECRETS');

		expect(screen.getAllByRole('option')).toHaveLength(1);
		expect(
			screen.getByRole('option', { name: /Settings: Secrets/ })
		).toBeInTheDocument();
	});

	it('should say so when nothing matches', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'zzzz');

		expect(screen.getByRole('status')).toHaveTextContent('No commands');
		expect(screen.queryByRole('option')).not.toBeInTheDocument();
		expect(getInput()).toHaveAttribute('aria-expanded', 'false');
	});

	it('should move the active option with the arrows, Home and End', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);
		const options = screen.getAllByRole('option');

		await user.keyboard('{ArrowDown}');
		expect(getInput()).toHaveAttribute('aria-activedescendant', options[1]?.id);

		await user.keyboard('{End}');
		expect(getInput()).toHaveAttribute(
			'aria-activedescendant',
			options.at(-1)?.id
		);

		await user.keyboard('{ArrowDown}');
		expect(getInput()).toHaveAttribute('aria-activedescendant', options[0]?.id);

		await user.keyboard('{ArrowUp}');
		expect(getInput()).toHaveAttribute(
			'aria-activedescendant',
			options.at(-1)?.id
		);

		await user.keyboard('{Home}');
		expect(getInput()).toHaveAttribute('aria-activedescendant', options[0]?.id);
	});

	it('should run the active command on Enter after closing', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.keyboard('{Enter}');

		await waitFor(() =>
			expect(useUiStore.getState().panel).toEqual({
				kind: 'tunnelEditor',
				tunnelId: null,
			})
		);
		expect(useUiStore.getState().isPaletteOpen).toBe(false);
	});

	it('should run a command clicked with the mouse', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.click(screen.getByRole('option', { name: /Import tunnels/ }));

		await waitFor(() =>
			expect(useUiStore.getState().panel).toEqual({ kind: 'import' })
		);
	});

	it('should start a tunnel through the daemon', async () => {
		const user = userEvent.setup();
		const { gateway } = renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'Start 3333 → nova:3333{Enter}');

		await waitFor(() =>
			expect(gateway.startTunnel).toHaveBeenCalledWith('tunnel-1')
		);
	});

	it('should send an awake tunnel back to standby through the daemon', async () => {
		const user = userEvent.setup();
		const { gateway } = renderPalette();
		useDaemonStore.setState({ tunnels: [buildTunnel({ isAwake: true })] });
		await openPalette(user);

		await user.type(getInput(), 'Send 3333 → nova:3333 to standby{Enter}');

		await waitFor(() =>
			expect(gateway.standbyTunnel).toHaveBeenCalledWith('tunnel-1')
		);
	});

	it('should ask for confirmation before deleting a tag', async () => {
		const user = userEvent.setup();
		const { gateway } = renderPalette();
		useDaemonStore.setState({ tunnels: [buildTunnel({ tags: ['dev'] })] });
		await openPalette(user);

		await user.type(getInput(), 'Delete tag dev{Enter}');

		expect(
			await screen.findByRole('alertdialog', { name: 'Delete tag dev?' })
		).toBeInTheDocument();
		expect(gateway.deleteTag).not.toHaveBeenCalled();
	});

	it('should open the host configuration of a host', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'Configuration of nova{Enter}');

		await waitFor(() =>
			expect(useUiStore.getState().panel).toEqual({
				kind: 'hostConfig',
				host: 'nova',
			})
		);
	});

	it('should open the settings on the tab of a settings command', async () => {
		const user = userEvent.setup();
		renderPalette();
		await openPalette(user);

		await user.type(getInput(), 'Settings: Standby{Enter}');

		await waitFor(() =>
			expect(useUiStore.getState().panel).toEqual({
				kind: 'settings',
				tab: 'standby',
			})
		);
		expect(
			await screen.findByRole('tab', { name: 'Standby', selected: true })
		).toBeInTheDocument();
	});

	it('should close on Escape and give the focus back', async () => {
		const user = userEvent.setup();
		renderPalette();
		screen.getByRole('button', { name: 'Trigger' }).focus();
		await openPalette(user);

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(useUiStore.getState().isPaletteOpen).toBe(false);
		expect(screen.getByRole('button', { name: 'Trigger' })).toHaveFocus();
	});
});
