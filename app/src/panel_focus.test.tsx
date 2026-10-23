import { ModalProvider } from '@minimalstuff/ui';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { Overlays } from '#/overlays';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';

const DETAILS_NAME = 'Tunnel 3333 → nova:3333';
const EDITOR_NAME = 'Edit 3333 → nova:3333';

const renderScreen = () => {
	useDaemonStore.setState({
		tunnels: [buildTunnel()],
		settings: buildSettings(),
	});
	return renderWithGateway(
		<>
			<main>
				<button
					onClick={() => useUiStore.getState().openTunnelDetails('tunnel-1')}
				>
					Details
				</button>
				<button onClick={() => useUiStore.getState().openImport()}>
					Import
				</button>
			</main>
			<ModalProvider />
			<Overlays />
		</>
	);
};

const openDetails = async () => {
	const user = userEvent.setup();
	renderScreen();
	await user.click(screen.getByRole('button', { name: 'Details' }));
	await screen.findByRole('dialog', { name: DETAILS_NAME });
	return user;
};

const switchToEditor = async (user: ReturnType<typeof userEvent.setup>) => {
	await user.click(screen.getByRole('button', { name: 'Edit' }));
	return screen.findByRole('dialog', { name: EDITOR_NAME });
};

describe('panel focus', () => {
	it('should keep the focus inside the editor when details switches to it', async () => {
		const user = await openDetails();

		const editor = await switchToEditor(user);
		await waitFor(() =>
			expect(
				screen.queryByRole('dialog', { name: DETAILS_NAME })
			).not.toBeInTheDocument()
		);

		expect(editor).toContainElement(document.activeElement as HTMLElement);
	});

	it('should put the focus on main when the editor closes after a switch', async () => {
		const user = await openDetails();
		await switchToEditor(user);
		await waitFor(() =>
			expect(
				screen.queryByRole('dialog', { name: DETAILS_NAME })
			).not.toBeInTheDocument()
		);

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(
				screen.queryByRole('dialog', { name: EDITOR_NAME })
			).not.toBeInTheDocument()
		);
		expect(document.querySelector('main')).toHaveFocus();
	});

	it('should return the focus to the opener when details closes', async () => {
		const user = await openDetails();

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(screen.getByRole('button', { name: 'Details' })).toHaveFocus();
	});

	it('should return the focus to the opener when the import drawer closes', async () => {
		const user = userEvent.setup();
		renderScreen();
		await user.click(screen.getByRole('button', { name: 'Import' }));
		await screen.findByRole('dialog', { name: 'Import tunnels' });

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(screen.getByRole('button', { name: 'Import' })).toHaveFocus();
	});

	it('should start a reopened editor from a fresh form', async () => {
		const user = userEvent.setup();
		renderScreen();
		act(() => useUiStore.getState().openTunnelEditor(null));
		const nameField = await screen.findByRole('textbox', {
			name: /^Listen port/,
		});
		await user.type(nameField, '1234');
		await user.keyboard('{Escape}');
		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);

		act(() => useUiStore.getState().openTunnelEditor(null));

		expect(
			await screen.findByRole('textbox', { name: /^Listen port/ })
		).toHaveValue('');
	});

	it('should have no axe violations while the details drawer is open', async () => {
		await openDetails();

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
