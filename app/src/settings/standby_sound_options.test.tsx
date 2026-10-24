import { fireEvent, screen, waitFor, within } from '@testing-library/react';
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
		volume?: number;
		onChange?: (sound: StandbySoundDto) => void;
		onVolumeChange?: (volume: number) => void;
	} = {},
	gateway = createFakeGateway()
) {
	return renderWithGateway(
		<StandbySoundOptions
			value={props.value ?? 'pluck'}
			notifyMode={props.notifyMode ?? 'both'}
			volume={props.volume ?? 50}
			onChange={props.onChange ?? vi.fn()}
			onVolumeChange={props.onVolumeChange ?? vi.fn()}
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

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke', 50);
	});

	it('should preview the selected sound for the failure when asked', async () => {
		const { gateway } = renderOptions({ value: 'system' });

		await userEvent.click(
			screen.getByRole('button', { name: 'Play the failure sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith(
			'system',
			'wakeFailed',
			50
		);
	});

	it('should show the volume as a labelled slider with its percentage', () => {
		renderOptions({ volume: 35 });

		const slider = screen.getByRole('slider', { name: 'Volume' });
		expect(slider).toHaveValue('35');
		expect(slider).toHaveAttribute('aria-valuetext', '35 %');
		expect(slider).toHaveAttribute('min', '5');
		expect(slider).toHaveAttribute('max', '100');
		expect(slider).toHaveAttribute('step', '5');
		expect(screen.getByText('35 %')).toBeInTheDocument();
	});

	it('should report the volume when the slider moves', () => {
		const onVolumeChange = vi.fn();
		renderOptions({ volume: 50, onVolumeChange });

		fireEvent.change(screen.getByRole('slider', { name: 'Volume' }), {
			target: { value: '75' },
		});

		expect(onVolumeChange).toHaveBeenCalledWith(75);
	});

	it.each<StandbyNotifyDto>(['none', 'visual'])(
		'should disable the volume when the signal is %s',
		(notifyMode) => {
			renderOptions({ notifyMode });

			expect(screen.getByRole('slider', { name: 'Volume' })).toBeDisabled();
		}
	);

	it.each<StandbyNotifyDto>(['sound', 'both'])(
		'should enable the volume when the signal is %s',
		(notifyMode) => {
			renderOptions({ notifyMode });

			expect(screen.getByRole('slider', { name: 'Volume' })).toBeEnabled();
		}
	);

	it('should preview at the draft volume when asked', async () => {
		const { gateway } = renderOptions({ value: 'marimba', volume: 25 });

		await userEvent.click(
			screen.getByRole('button', { name: 'Play the wake sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('marimba', 'woke', 25);
	});

	it('should replace the volume with a hint when the sound is the system one', () => {
		renderOptions({ value: 'system' });

		expect(
			screen.queryByRole('slider', { name: 'Volume' })
		).not.toBeInTheDocument();
		expect(
			screen.getByRole('combobox', { name: 'Sound' })
		).toHaveAccessibleDescription(
			'The system sound follows the system volume.'
		);
	});

	it('should have no axe violations with the system sound', async () => {
		const { container } = renderOptions({ value: 'system' });

		expect(await axe(container)).toHaveNoViolations();
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
