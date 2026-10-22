import { axe } from 'vitest-axe';
import { describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { StandbySoundOptions } from '#/settings/standby_sound_options';
import type { StandbyNotifyDto, StandbySoundDto } from '#/daemon/protocol';

const HINT = /Choisissez « Son » ou « Les deux »/;

function renderOptions(
	props: {
		value?: StandbySoundDto;
		notifyMode?: StandbyNotifyDto;
		onChange?: (sound: StandbySoundDto) => void;
	} = {},
	gateway = createFakeGateway()
) {
	return renderWithGateway(
		<StandbySoundOptions
			value={props.value ?? 'pluck'}
			notifyMode={props.notifyMode ?? 'both'}
			onChange={props.onChange ?? vi.fn()}
		/>,
		gateway
	);
}

describe('StandbySoundOptions', () => {
	it('should render the four sounds in order with the selected one chosen', () => {
		renderOptions({ value: 'marimba' });

		const select = screen.getByRole('combobox', { name: 'Son' });
		expect(
			within(select)
				.getAllByRole('option')
				.map((option) => option.getAttribute('value'))
		).toEqual(['pluck', 'chime', 'marimba', 'system']);
		expect(select).toHaveValue('marimba');
	});

	it('should report the chosen sound when an option is picked', async () => {
		const onChange = vi.fn();
		renderOptions({ onChange });

		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Son' }),
			'Carillon'
		);

		expect(onChange).toHaveBeenCalledWith('chime');
	});

	it.each<StandbyNotifyDto>(['none', 'visual'])(
		'should hint that sound is off when the signal is %s',
		(notifyMode) => {
			renderOptions({ notifyMode });

			expect(
				screen.getByRole('combobox', { name: 'Son' })
			).toHaveAccessibleDescription(HINT);
		}
	);

	it.each<StandbyNotifyDto>(['sound', 'both'])(
		'should not hint that sound is off when the signal is %s',
		(notifyMode) => {
			renderOptions({ notifyMode });

			expect(screen.queryByText(HINT)).not.toBeInTheDocument();
		}
	);

	it('should describe the system sound', () => {
		renderOptions();

		expect(
			screen.getByRole('combobox', { name: 'Son' })
		).toHaveAccessibleDescription(/schéma de sons de Windows/);
	});

	it('should preview the selected sound for the wake when asked', async () => {
		const { gateway } = renderOptions({ value: 'chime' });

		await userEvent.click(
			screen.getByRole('button', { name: 'Écouter le réveil' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke');
	});

	it('should preview the selected sound for the failure when asked', async () => {
		const { gateway } = renderOptions({ value: 'system' });

		await userEvent.click(
			screen.getByRole('button', { name: "Écouter l'échec" })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('system', 'wakeFailed');
	});

	it('should toast an error when the preview fails', async () => {
		const gateway = createFakeGateway();
		gateway.previewSound.mockRejectedValue(
			new DaemonCommandError('internal', 'pas de sortie audio')
		);
		renderOptions({}, gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Écouter le réveil' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: 'Écoute impossible : pas de sortie audio',
				}),
			])
		);
	});

	it('should have no axe violations', async () => {
		const { container } = renderOptions({ notifyMode: 'none' });

		expect(await axe(container)).toHaveNoViolations();
	});
});
