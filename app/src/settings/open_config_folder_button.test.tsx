import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';

const BUTTON_NAME = 'Ouvrir le dossier de configuration';

describe('OpenConfigFolderButton', () => {
	it('should reveal the configuration folder', async () => {
		const { gateway } = renderWithGateway(<OpenConfigFolderButton />);

		await userEvent.click(screen.getByRole('button', { name: BUTTON_NAME }));

		expect(gateway.revealConfig).toHaveBeenCalledOnce();
	});

	it('should toast an error when it cannot open', async () => {
		const gateway = createFakeGateway();
		gateway.revealConfig.mockRejectedValue(
			new DaemonCommandError('internal', 'pas de gestionnaire')
		);
		renderWithGateway(<OpenConfigFolderButton />, gateway);

		await userEvent.click(screen.getByRole('button', { name: BUTTON_NAME }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: 'Ouverture impossible : pas de gestionnaire',
				}),
			])
		);
	});

	it('should have no axe violations', async () => {
		const { container } = renderWithGateway(<OpenConfigFolderButton />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
