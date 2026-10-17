import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { HostConfigButton } from '#/host_config/host_config_button';

describe('HostConfigButton', () => {
	it('should open the configuration panel of its host', async () => {
		render(<HostConfigButton host="nova" />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Configuration de nova' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'hostConfig',
			host: 'nova',
		});
	});

	it('should have no axe violations', async () => {
		const { container } = render(<HostConfigButton host="nova" />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
