import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';

import { useUiStore } from '#/state/ui_store';
import { HeaderActions } from '#/header/header_actions';
import { renderWithGateway } from '#/testing/render_with_gateway';

const renderActions = () =>
	renderWithGateway(
		<>
			<ModalProvider />
			<HeaderActions />
		</>
	);

describe('HeaderActions', () => {
	it('should open the tunnel editor in create mode', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(screen.getByRole('button', { name: 'Nouveau tunnel' }));

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should open the import panel', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(screen.getByRole('button', { name: 'Importer' }));

		expect(useUiStore.getState().panel).toEqual({ kind: 'import' });
	});

	it('should open the command palette and advertise its shortcut', async () => {
		const user = userEvent.setup();
		renderActions();
		const button = screen.getByRole('button', { name: 'Palette de commandes' });

		await user.click(button);

		expect(useUiStore.getState().isPaletteOpen).toBe(true);
		expect(button).toHaveAttribute('aria-keyshortcuts', 'Control+K');
	});

	it('should show the shortcut in the tooltip of the palette button', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.hover(
			screen.getByRole('button', { name: 'Palette de commandes' })
		);

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			'Palette de commandes (Ctrl+K)'
		);
	});

	it('should show the current theme in the tooltip of the theme toggle', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.hover(screen.getByRole('button', { name: /^Thème : / }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			/^Thème : .+, changer$/
		);
	});

	it('should open the settings', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(screen.getByRole('button', { name: 'Paramètres' }));

		expect(useUiStore.getState().panel.kind).toBe('settings');
	});

	it('should open the shortcuts help', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(
			screen.getByRole('button', { name: 'Raccourcis clavier' })
		);

		expect(useUiStore.getState().isHelpOpen).toBe(true);
	});

	it('should name the theme toggle in French', () => {
		renderActions();

		expect(
			screen.getByRole('button', { name: /^Thème : .+, changer$/ })
		).toBeInTheDocument();
	});

	it('should offer to quit', () => {
		renderActions();

		expect(
			screen.getByRole('button', { name: 'Quitter Tunneler' })
		).toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		const { container } = renderActions();

		expect(await axe(container)).toHaveNoViolations();
	});
});
