import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { Drawer } from '#/shared/drawer';
import { renderWithI18n } from '#/testing/render_with_i18n';

function Harness() {
	const [isOpen, setIsOpen] = useState(false);
	const [isTriggerShown, setIsTriggerShown] = useState(true);

	return (
		<>
			<main tabIndex={-1} aria-label="Contenu">
				{isTriggerShown && (
					<button
						onClick={() => {
							setIsOpen(true);
							setIsTriggerShown(false);
						}}
					>
						Ouvrir
					</button>
				)}
			</main>
			<Drawer isOpen={isOpen} title="Edit" onClose={() => setIsOpen(false)}>
				<input aria-label="Nom" />
			</Drawer>
		</>
	);
}

describe('useDrawerFocus', () => {
	it('should focus the main landmark when the previously focused element is gone', async () => {
		const user = userEvent.setup();
		renderWithI18n(<Harness />);
		await user.click(screen.getByRole('button', { name: 'Ouvrir' }));

		await user.click(screen.getByRole('button', { name: 'Close' }));

		expect(screen.getByRole('main')).toHaveFocus();
	});

	it('should focus the main landmark when nothing was focused before opening', async () => {
		const user = userEvent.setup();
		function BodyHarness() {
			const [isOpen, setIsOpen] = useState(true);
			return (
				<>
					<main tabIndex={-1} />
					<Drawer isOpen={isOpen} title="Edit" onClose={() => setIsOpen(false)}>
						<input aria-label="Nom" />
					</Drawer>
				</>
			);
		}
		renderWithI18n(<BodyHarness />);

		await user.click(screen.getByRole('button', { name: 'Close' }));

		expect(screen.getByRole('main')).toHaveFocus();
	});
});
