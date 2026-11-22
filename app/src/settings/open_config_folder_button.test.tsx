import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { getShownToasts } from '#/testing/shown_toasts';

const BUTTON_NAME = 'Open the configuration folder';

describe('OpenConfigFolderButton', () => {
	it('should reveal the configuration folder', async () => {
		const { gateway } = renderWithGateway(<OpenConfigFolderButton />);

		await userEvent.click(screen.getByRole('button', { name: BUTTON_NAME }));

		expect(gateway.revealConfig).toHaveBeenCalledOnce();
	});

	it('should toast an error when it cannot open', async () => {
		const gateway = createFakeGateway();
		gateway.revealConfig.mockRejectedValue(
			new DaemonCommandError('no_handler', 'no file manager')
		);
		renderWithGateway(<OpenConfigFolderButton />, gateway);

		await userEvent.click(screen.getByRole('button', { name: BUTTON_NAME }));

		await waitFor(() =>
			expect(getShownToasts()).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: "Couldn't open the folder: no file manager",
				}),
			])
		);
	});

	it('should have no axe violations', async () => {
		const { container } = renderWithGateway(<OpenConfigFolderButton />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
