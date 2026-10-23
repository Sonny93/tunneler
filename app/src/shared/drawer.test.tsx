import { useGlobalHotkeysStore } from '@minimalstuff/ui';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { Drawer } from '#/shared/drawer';
import { renderWithI18n } from '#/testing/render_with_i18n';

function DrawerHarness({
	hasInputs = true,
}: Readonly<{ hasInputs?: boolean }>) {
	const [isOpen, setIsOpen] = useState(false);
	const open = () => setIsOpen(true);
	const close = () => setIsOpen(false);

	return (
		<>
			<button onClick={open}>Ouvrir</button>
			<Drawer
				isOpen={isOpen}
				title="Edit"
				onClose={close}
				footer={<button>Enregistrer</button>}
			>
				{hasInputs ? <input aria-label="Nom" /> : <p>Contenu</p>}
			</Drawer>
		</>
	);
}

const openDrawer = async (hasInputs = true) => {
	const user = userEvent.setup();
	renderWithI18n(<DrawerHarness hasInputs={hasInputs} />);
	await user.click(screen.getByRole('button', { name: 'Ouvrir' }));
	return user;
};

describe('Drawer', () => {
	it('should render nothing when closed', () => {
		renderWithI18n(
			<Drawer isOpen={false} title="Edit" onClose={vi.fn()}>
				x
			</Drawer>
		);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should be a modal dialog labelled by its title', async () => {
		await openDrawer();

		const dialog = screen.getByRole('dialog', { name: 'Edit' });
		expect(dialog).toHaveAttribute('aria-modal', 'true');
	});

	it('should title the dialog with a level 2 heading', async () => {
		await openDrawer();

		expect(
			screen.getByRole('heading', { name: 'Edit', level: 2 })
		).toBeInTheDocument();
	});

	it('should move the focus to the first focusable element of the content', async () => {
		await openDrawer();

		expect(screen.getByRole('textbox', { name: 'Nom' })).toHaveFocus();
	});

	it('should move the focus to the panel when the content has nothing focusable', async () => {
		await openDrawer(false);

		expect(screen.getByRole('dialog')).toHaveFocus();
	});

	it('should render the footer', async () => {
		await openDrawer();

		expect(
			screen.getByRole('button', { name: 'Enregistrer' })
		).toBeInTheDocument();
	});

	it('should close on Escape, return the focus to the opener and restore the global hotkeys', async () => {
		const user = await openDrawer();
		const whileOpen = useGlobalHotkeysStore.getState().globalHotkeysEnabled;

		await user.keyboard('{Escape}');

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Ouvrir' })).toHaveFocus();
		expect(whileOpen).toBe(false);
		expect(useGlobalHotkeysStore.getState().globalHotkeysEnabled).toBe(true);
	});

	it('should close with the close button', async () => {
		const user = await openDrawer();

		await user.click(screen.getByRole('button', { name: 'Close' }));

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should close when the backdrop is clicked', async () => {
		const user = await openDrawer();

		await user.click(screen.getByTestId('drawer-backdrop'));

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should wrap Tab from the last focusable element to the first', async () => {
		const user = await openDrawer();
		screen.getByRole('button', { name: 'Enregistrer' }).focus();

		await user.tab();

		expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus();
	});

	it('should wrap Shift+Tab from the first focusable element to the last', async () => {
		const user = await openDrawer();
		screen.getByRole('button', { name: 'Close' }).focus();

		await user.tab({ shift: true });

		expect(screen.getByRole('button', { name: 'Enregistrer' })).toHaveFocus();
	});

	it('should have no axe violations when open', async () => {
		await openDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
