import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, render, screen, waitFor } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { ShortcutsHelp } from '#/shortcuts/shortcuts_help';

function renderHelp() {
	return render(
		<>
			<button>Déclencheur</button>
			<ModalProvider />
			<ShortcutsHelp />
		</>
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
			name: 'Raccourcis clavier',
		});
		for (const text of [
			'n',
			'Nouveau tunnel',
			',',
			'?',
			'Importer des tunnels',
			'Ctrl',
			'K',
			'Ouvrir la palette de commandes',
			'Tunnel précédent / suivant',
			'Démarrer ou arrêter le tunnel, cocher ou décocher la case',
			'Maj',
			'clic',
			'Sélectionner une plage de tunnels',
			'A',
			'Sélectionner tous les tunnels affichés',
			'Effacer la sélection',
			'Fermer le panneau ou la fenêtre',
			'Toutes ces actions sont aussi disponibles avec les boutons.',
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

		await user.click(await screen.findByRole('button', { name: 'Fermer' }));

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
		screen.getByRole('button', { name: 'Déclencheur' }).focus();
		openHelp();
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(screen.getByRole('button', { name: 'Déclencheur' })).toHaveFocus()
		);
	});
});
