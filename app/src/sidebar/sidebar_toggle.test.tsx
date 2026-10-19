import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { SIDEBAR_ID } from '#/sidebar/sidebar_id';
import { SidebarToggle } from '#/sidebar/sidebar_toggle';

function Harness() {
	const isCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	return (
		<>
			<SidebarToggle />
			<aside hidden={isCollapsed}>
				<nav id={SIDEBAR_ID} aria-label="Filtres">
					<button>Filtre</button>
				</nav>
			</aside>
		</>
	);
}

describe('SidebarToggle', () => {
	it('should offer to hide the expanded sidebar', () => {
		render(<Harness />);

		const button = screen.getByRole('button', { name: 'Masquer le panneau' });

		expect(button).toHaveAttribute('aria-expanded', 'true');
		expect(button).toHaveAttribute('aria-controls', SIDEBAR_ID);
	});

	it('should offer to show the collapsed sidebar', async () => {
		render(<Harness />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Masquer le panneau' })
		);

		expect(
			screen.getByRole('button', { name: 'Afficher le panneau' })
		).toHaveAttribute('aria-expanded', 'false');
	});

	it('should move the focus to the toggle when it hides the focused sidebar', async () => {
		const user = userEvent.setup();
		render(<Harness />);
		await user.click(screen.getByRole('button', { name: 'Filtre' }));

		useUiStore.getState().toggleSidebar();

		expect(
			await screen.findByRole('button', { name: 'Afficher le panneau' })
		).toHaveFocus();
	});

	it('should show the shortcut in the tooltip', async () => {
		const user = userEvent.setup();
		render(<Harness />);

		await user.hover(
			screen.getByRole('button', { name: 'Masquer le panneau' })
		);

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			'Masquer le panneau (B)'
		);
	});

	it('should have no axe violations', async () => {
		const { container } = render(<Harness />);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations while the tooltip is shown', async () => {
		const user = userEvent.setup();
		render(
			<main>
				<Harness />
			</main>
		);
		await user.tab();
		await screen.findByRole('tooltip');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
