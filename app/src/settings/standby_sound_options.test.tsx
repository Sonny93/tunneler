import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { StandbyNotifyDto, StandbySoundDto } from '#/daemon/protocol';
import { StandbySoundOptions } from '#/settings/standby_sound_options';
import { useToastStore } from '#/shared/toasts/toast_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

const HINT = /Not played with the chosen signal/;

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

		const select = screen.getByRole('combobox', { name: 'Sound' });
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
			screen.getByRole('combobox', { name: 'Sound' }),
			'Chime'
		);

		expect(onChange).toHaveBeenCalledWith('chime');
	});

	it.each<StandbyNotifyDto>(['none', 'visual'])(
		'should hint that sound is off when the signal is %s',
		(notifyMode) => {
			renderOptions({ notifyMode });

			expect(
				screen.getByRole('combobox', { name: 'Sound' })
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

	it('should preview the selected sound for the wake when asked', async () => {
		const { gateway } = renderOptions({ value: 'chime' });

		await userEvent.click(
			screen.getByRole('button', { name: 'Play the wake sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke');
	});

	it('should preview the selected sound for the failure when asked', async () => {
		const { gateway } = renderOptions({ value: 'system' });

		await userEvent.click(
			screen.getByRole('button', { name: 'Play the failure sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('system', 'wakeFailed');
	});

	it('should toast an error when the preview fails', async () => {
		const gateway = createFakeGateway();
		gateway.previewSound.mockRejectedValue(
			new DaemonCommandError('audio_unavailable', 'no audio output')
		);
		renderOptions({}, gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Play the wake sound' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: "Couldn't play the sound: no audio output",
				}),
			])
		);
	});

	it('should have no axe violations', async () => {
		const { container } = renderOptions({ notifyMode: 'none' });

		expect(await axe(container)).toHaveNoViolations();
	});
});
