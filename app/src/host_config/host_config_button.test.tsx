import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { HostConfigButton } from '#/host_config/host_config_button';
import { useUiStore } from '#/state/ui_store';
import { renderWithI18n } from '#/testing/render_with_i18n';

describe('HostConfigButton', () => {
	it('should open the configuration panel of its host', async () => {
		renderWithI18n(<HostConfigButton host="nova" />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Configuration of nova' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'hostConfig',
			host: 'nova',
		});
	});

	it('should have no axe violations', async () => {
		const { container } = renderWithI18n(<HostConfigButton host="nova" />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
