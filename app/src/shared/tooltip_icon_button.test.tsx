import { axe } from 'vitest-axe';
import { describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';

import { TooltipIconButton } from '#/shared/tooltip_icon_button';

describe('TooltipIconButton', () => {
	it('should show its name as a tooltip on hover', async () => {
		render(<TooltipIconButton icon="i-mdi-cog" aria-label="Paramètres" />);

		await userEvent.hover(screen.getByRole('button', { name: 'Paramètres' }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			/^Paramètres$/
		);
	});

	it('should append the shortcut to the tooltip', async () => {
		render(
			<TooltipIconButton
				icon="i-mdi-cog"
				aria-label="Paramètres"
				shortcutHint=","
			/>
		);

		await userEvent.hover(screen.getByRole('button', { name: 'Paramètres' }));

		expect(await screen.findByRole('tooltip')).toHaveTextContent(
			'Paramètres (,)'
		);
	});

	it('should show the tooltip on keyboard focus', async () => {
		render(<TooltipIconButton icon="i-mdi-cog" aria-label="Paramètres" />);

		await userEvent.tab();

		expect(await screen.findByRole('tooltip')).toBeInTheDocument();
	});

	it('should dismiss the tooltip with Escape', async () => {
		render(<TooltipIconButton icon="i-mdi-cog" aria-label="Paramètres" />);
		await userEvent.tab();
		await screen.findByRole('tooltip');

		await userEvent.keyboard('{Escape}');

		await vi.waitFor(() =>
			expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
		);
	});

	it('should keep the accessible name unchanged by the shortcut', () => {
		render(
			<TooltipIconButton
				icon="i-mdi-cog"
				aria-label="Paramètres"
				shortcutHint=","
			/>
		);

		expect(
			screen.getByRole('button', { name: 'Paramètres' })
		).toBeInTheDocument();
	});

	it('should forward the click to the button', async () => {
		const onClick = vi.fn();
		render(
			<TooltipIconButton
				icon="i-mdi-cog"
				aria-label="Paramètres"
				onClick={onClick}
			/>
		);

		await userEvent.click(screen.getByRole('button', { name: 'Paramètres' }));

		expect(onClick).toHaveBeenCalledOnce();
	});

	it('should have no axe violations while the tooltip is shown', async () => {
		render(
			<main>
				<TooltipIconButton icon="i-mdi-cog" aria-label="Paramètres" />
			</main>
		);
		await userEvent.tab();
		await screen.findByRole('tooltip');

		expect(
			await axe(document.body, { rules: { region: { enabled: false } } })
		).toHaveNoViolations();
	});
});
