import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { useUiStore } from '#/state/ui_store';
import { renderWithI18n } from '#/testing/render_with_i18n';
import { TunnelListEmpty } from '#/tunnels/list/tunnel_list_empty';

describe('TunnelListEmpty', () => {
	it('should open the editor in create mode', async () => {
		renderWithI18n(<TunnelListEmpty createButtonRef={null} />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Create a tunnel' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should have no accessibility violation', async () => {
		const { container } = renderWithI18n(
			<TunnelListEmpty createButtonRef={null} />
		);

		expect(await axe(container)).toHaveNoViolations();
	});
});
