import { ModalProvider } from '@minimalstuff/ui';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { HeaderActions } from '#/header/header_actions';
import { useUiStore } from '#/state/ui_store';
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

		await user.click(screen.getByRole('button', { name: 'New tunnel' }));

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should reach the chevron with Tab right after the main button', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.tab();
		await user.tab();

		expect(
			screen.getByRole('button', { name: 'Other ways to create a tunnel' })
		).toHaveFocus();
	});

	it('should announce a menu on the chevron', () => {
		renderActions();

		expect(
			screen.getByRole('button', { name: 'Other ways to create a tunnel' })
		).toHaveAttribute('aria-haspopup');
	});

	it.each(['{Enter}', '{ArrowDown}'])(
		'should open the menu from the chevron with %s',
		async (key) => {
			const user = userEvent.setup();
			renderActions();
			screen
				.getByRole('button', { name: 'Other ways to create a tunnel' })
				.focus();

			await user.keyboard(key);

			expect(
				await screen.findByRole('menuitem', { name: 'Temporary tunnel…' })
			).toBeInTheDocument();
		}
	);

	it('should open the temporary tunnel form from the menu', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(
			screen.getByRole('button', { name: 'Other ways to create a tunnel' })
		);
		await user.click(
			await screen.findByRole('menuitem', { name: 'Temporary tunnel…' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'temporaryTunnelEditor',
		});
	});

	it('should have no axe violations with the menu open', async () => {
		const user = userEvent.setup();
		const { container } = renderActions();
		await user.click(
			screen.getByRole('button', { name: 'Other ways to create a tunnel' })
		);
		await screen.findByRole('menuitem', { name: 'Temporary tunnel…' });

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should open the documentation home from the help button', async () => {
		const user = userEvent.setup();
		const { gateway } = renderActions();

		await user.click(screen.getByRole('button', { name: 'Documentation' }));

		expect(gateway.openDocumentation).toHaveBeenCalledWith('home', 'en');
	});

	it('should open the import panel', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(screen.getByRole('button', { name: 'Import' }));

		expect(useUiStore.getState().panel).toEqual({ kind: 'import' });
	});

	it('should open the command palette and advertise its shortcut', async () => {
		const user = userEvent.setup();
		renderActions();
		const button = screen.getByRole('button', { name: 'Command palette' });

		await user.click(button);

		expect(useUiStore.getState().isPaletteOpen).toBe(true);
		expect(button).toHaveAttribute('aria-keyshortcuts', 'Control+K');
	});

	it('should show the shortcut in the tooltip of the palette button', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.hover(screen.getByRole('button', { name: 'Command palette' }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			'Command palette (Ctrl+K)'
		);
	});

	it('should show the current theme in the tooltip of the theme toggle', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.hover(screen.getByRole('button', { name: /^Theme: / }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			/^Theme: .+, change$/
		);
	});

	it('should open the settings', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(screen.getByRole('button', { name: 'Settings' }));

		expect(useUiStore.getState().panel.kind).toBe('settings');
	});

	it('should open the shortcuts help', async () => {
		const user = userEvent.setup();
		renderActions();

		await user.click(
			screen.getByRole('button', { name: 'Keyboard shortcuts' })
		);

		expect(useUiStore.getState().isHelpOpen).toBe(true);
	});

	it('should name the theme toggle after the current theme', () => {
		renderActions();

		expect(
			screen.getByRole('button', { name: /^Theme: .+, change$/ })
		).toBeInTheDocument();
	});

	it('should offer to quit', () => {
		renderActions();

		expect(
			screen.getByRole('button', { name: 'Quit Tunneler' })
		).toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		const { container } = renderActions();

		expect(await axe(container)).toHaveNoViolations();
	});
});
