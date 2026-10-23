import { useDisableHotkeysWhileMounted } from '@minimalstuff/ui';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { useGlobalShortcuts } from '#/shortcuts/use_global_shortcuts';
import { useUiStore } from '#/state/ui_store';
import { renderWithGateway as render } from '#/testing/render_with_gateway';

function Harness({ isModalOpen = false }: Readonly<{ isModalOpen?: boolean }>) {
	useGlobalShortcuts();
	return (
		<>
			<input aria-label="Field" />
			<textarea aria-label="Area" />
			<div
				contentEditable
				suppressContentEditableWarning
				aria-label="Editable"
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

	it('should open the import on i', async () => {
		render(<Harness />);

		await userEvent.keyboard('i');

		expect(useUiStore.getState().panel).toEqual({ kind: 'import' });
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

	it('should open the documentation home on F1', async () => {
		const { gateway } = render(<Harness />);

		await userEvent.keyboard('{F1}');

		expect(gateway.openDocumentation).toHaveBeenCalledWith('home', 'en');
	});

	it('should toggle the sidebar on b', async () => {
		render(<Harness />);

		await userEvent.keyboard('b');

		expect(useUiStore.getState().isSidebarCollapsed).toBe(true);
	});

	it('should not toggle the sidebar on b while a dialog is open', async () => {
		render(<Harness isModalOpen />);

		await userEvent.keyboard('b');

		expect(useUiStore.getState().isSidebarCollapsed).toBe(false);
	});

	it('should not toggle the sidebar on b typed in a field', async () => {
		const user = userEvent.setup();
		render(<Harness />);
		await user.click(screen.getByLabelText('Field'));

		await user.keyboard('b');

		expect(useUiStore.getState().isSidebarCollapsed).toBe(false);
	});

	it('should not toggle the sidebar on Ctrl+b', async () => {
		render(<Harness />);

		await userEvent.keyboard('{Control>}b{/Control}');

		expect(useUiStore.getState().isSidebarCollapsed).toBe(false);
	});

	it.each(['{Control>}k{/Control}', '{Meta>}k{/Meta}'])(
		'should open the palette on %s',
		async (keys) => {
			render(<Harness />);

			await userEvent.keyboard(keys);

			expect(useUiStore.getState().isPaletteOpen).toBe(true);
		}
	);

	it('should open the palette even from a text field', async () => {
		const user = userEvent.setup();
		render(<Harness />);
		await user.click(screen.getByLabelText('Field'));

		await user.keyboard('{Control>}k{/Control}');

		expect(useUiStore.getState().isPaletteOpen).toBe(true);
	});

	it('should not open the palette on k alone', async () => {
		render(<Harness />);

		await userEvent.keyboard('k');

		expect(useUiStore.getState().isPaletteOpen).toBe(false);
	});

	it('should not open the palette while a dialog disables the global hotkeys', async () => {
		render(<Harness isModalOpen />);

		await userEvent.keyboard('{Control>}k{/Control}');

		expect(useUiStore.getState().isPaletteOpen).toBe(false);
	});

	it('should ignore other keys', async () => {
		render(<Harness />);

		await userEvent.keyboard('x');

		expect(useUiStore.getState().panel.kind).toBe('none');
	});

	it.each(['Field', 'Area', 'Editable'])(
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
