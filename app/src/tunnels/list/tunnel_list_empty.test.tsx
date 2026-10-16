import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { TunnelListEmpty } from '#/tunnels/list/tunnel_list_empty';

describe('TunnelListEmpty', () => {
	it('should open the editor in create mode', async () => {
		render(<TunnelListEmpty createButtonRef={null} />);

		await userEvent.click(
			screen.getByRole('button', { name: 'Créer un tunnel' })
		);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should have no accessibility violation', async () => {
		const { container } = render(<TunnelListEmpty createButtonRef={null} />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
