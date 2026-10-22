import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { useUiStore } from '#/state/ui_store';
import { SIDEBAR_ID } from '#/sidebar/sidebar_id';
import { SidebarToggle } from '#/sidebar/sidebar_toggle';
import { renderWithI18n } from '#/testing/render_with_i18n';

function Harness() {
	const isCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	return (
		<>
			<SidebarToggle />
			<aside hidden={isCollapsed}>
				<nav id={SIDEBAR_ID} aria-label="Filters">
					<button>Filter</button>
				</nav>
			</aside>
		</>
	);
}

describe('SidebarToggle', () => {
	it('should offer to hide the expanded sidebar', () => {
		renderWithI18n(<Harness />);

		const button = screen.getByRole('button', { name: 'Hide the sidebar' });

		expect(button).toHaveAttribute('aria-expanded', 'true');
		expect(button).toHaveAttribute('aria-controls', SIDEBAR_ID);
	});

	it('should offer to show the collapsed sidebar', async () => {
		renderWithI18n(<Harness />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Hide the sidebar' })
		);

		expect(
			screen.getByRole('button', { name: 'Show the sidebar' })
		).toHaveAttribute('aria-expanded', 'false');
	});

	it('should move the focus to the toggle when it hides the focused sidebar', async () => {
		const user = userEvent.setup();
		renderWithI18n(<Harness />);
		await user.click(screen.getByRole('button', { name: 'Filter' }));

		useUiStore.getState().toggleSidebar();

		expect(
			await screen.findByRole('button', { name: 'Show the sidebar' })
		).toHaveFocus();
	});

	it('should show the shortcut in the tooltip', async () => {
		const user = userEvent.setup();
		renderWithI18n(<Harness />);

		await user.hover(screen.getByRole('button', { name: 'Hide the sidebar' }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			'Hide the sidebar (B)'
		);
	});

	it('should have no axe violations', async () => {
		const { container } = renderWithI18n(<Harness />);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations while the tooltip is shown', async () => {
		const user = userEvent.setup();
		renderWithI18n(
			<main>
				<Harness />
			</main>
		);
		await user.tab();
		await screen.findByRole('tooltip');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
