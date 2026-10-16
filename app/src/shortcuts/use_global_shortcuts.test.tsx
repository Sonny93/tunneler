import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';
import { useDisableHotkeysWhileMounted } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { useGlobalShortcuts } from '#/shortcuts/use_global_shortcuts';

function Harness({ isModalOpen = false }: Readonly<{ isModalOpen?: boolean }>) {
	useGlobalShortcuts();
	return (
		<>
			<input aria-label="Champ" />
			<textarea aria-label="Zone" />
			<div
				contentEditable
				suppressContentEditableWarning
				aria-label="Éditable"
			/>
			{isModalOpen && <OpenModalStandIn />}
		</>
	);
}

function OpenModalStandIn() {
	useDisableHotkeysWhileMounted();
	return null;
}

describe('useGlobalShortcuts', () => {
	it('should open the tunnel editor on n', async () => {
		render(<Harness />);

		await userEvent.keyboard('n');

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should open the settings on comma', async () => {
		render(<Harness />);

		await userEvent.keyboard(',');

		expect(useUiStore.getState().panel.kind).toBe('settings');
	});

	it('should open the help on question mark', async () => {
		render(<Harness />);

		await userEvent.keyboard('?');

		expect(useUiStore.getState().isHelpOpen).toBe(true);
	});

	it('should ignore other keys', async () => {
		render(<Harness />);

		await userEvent.keyboard('x');

		expect(useUiStore.getState().panel.kind).toBe('none');
	});

	it.each(['Champ', 'Zone', 'Éditable'])(
		'should ignore keys typed in "%s"',
		async (name) => {
			const user = userEvent.setup();
			render(<Harness />);

			await user.click(screen.getByLabelText(name));
			await user.keyboard('n');

			expect(useUiStore.getState().panel.kind).toBe('none');
		}
	);

	it.each(['{Control>}n{/Control}', '{Alt>}n{/Alt}', '{Meta>}n{/Meta}'])(
		'should ignore %s',
		async (keys) => {
			render(<Harness />);

			await userEvent.keyboard(keys);

			expect(useUiStore.getState().panel.kind).toBe('none');
		}
	);

	it('should ignore keys while a dialog disables the global hotkeys', async () => {
		render(<Harness isModalOpen />);

		await userEvent.keyboard('n');

		expect(useUiStore.getState().panel.kind).toBe('none');
	});

	it('should stop listening once unmounted', async () => {
		const { unmount } = render(<Harness />);
		unmount();

		await userEvent.keyboard('n');

		expect(useUiStore.getState().panel.kind).toBe('none');
	});
});
