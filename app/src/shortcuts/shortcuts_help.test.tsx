import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { AppUIProvider } from '#/shared/app_ui_provider';
import { ShortcutsHelp } from '#/shortcuts/shortcuts_help';
import { renderWithI18n } from '#/testing/render_with_i18n';

function renderHelp() {
	return renderWithI18n(
		<AppUIProvider>
			<button>Trigger</button>
			<ModalProvider />
			<ShortcutsHelp />
		</AppUIProvider>
	);
}

const openHelp = () => act(() => useUiStore.getState().setHelpOpen(true));

describe('ShortcutsHelp', () => {
	it('should render nothing while closed', () => {
		renderHelp();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should list every shortcut with its key', async () => {
		renderHelp();

		openHelp();

		const dialog = await screen.findByRole('dialog', {
			name: 'Keyboard shortcuts',
		});
		for (const text of [
			'n',
			'New tunnel',
			',',
			'?',
			'Import tunnels',
			'Ctrl',
			'K',
			'Open the command palette',
			'Previous / next tunnel',
			'Start or stop the tunnel, check or uncheck the checkbox',
			'Shift',
			'click',
			'Select a range of tunnels',
			'A',
			'Select all displayed tunnels',
			'Clear the selection',
			'Close the panel or the dialog',
			'All these actions are also available with the buttons.',
		]) {
			expect(dialog).toHaveTextContent(text, { normalizeWhitespace: true });
		}
	});

	it('should have no axe violations', async () => {
		renderHelp();
		openHelp();
		await screen.findByRole('dialog');

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should close with the close button and reset the state', async () => {
		const user = userEvent.setup();
		renderHelp();
		openHelp();

		await user.click(await screen.findByRole('button', { name: 'Close' }));

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(useUiStore.getState().isHelpOpen).toBe(false);
	});

	it('should close on Escape', async () => {
		const user = userEvent.setup();
		renderHelp();
		openHelp();
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() => expect(useUiStore.getState().isHelpOpen).toBe(false));
	});

	it('should return the focus to the trigger on close', async () => {
		const user = userEvent.setup();
		renderHelp();
		screen.getByRole('button', { name: 'Trigger' }).focus();
		openHelp();
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(screen.getByRole('button', { name: 'Trigger' })).toHaveFocus()
		);
	});
});
