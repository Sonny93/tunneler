import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { Button, Select } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import type {
	SoundEventDto,
	StandbyNotifyDto,
	StandbySoundDto,
} from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { translateOptions } from '#/i18n/translate_options';
import { StandbyVolumeSlider } from '#/settings/standby_volume_slider';

const OPTIONS: readonly {
	value: StandbySoundDto;
	label: MessageDescriptor;
}[] = [
	{ value: 'pluck', label: msg`Plucked string` },
	{ value: 'chime', label: msg`Chime` },
	{ value: 'marimba', label: msg`Marimba` },
	{ value: 'system', label: msg`System sound` },
];

const SILENT_MODES: readonly StandbyNotifyDto[] = ['none', 'visual'];

type StandbySoundOptionsProps = {
	value: StandbySoundDto;
	notifyMode: StandbyNotifyDto;
	volume: number;
	onChange: (sound: StandbySoundDto) => void;
	onVolumeChange: (volume: number) => void;
};

/** Which sound signals a wake or a failed wake, with a preview of both. */
export function StandbySoundOptions({
	value,
	notifyMode,
	volume,
	onChange,
	onVolumeChange,
}: Readonly<StandbySoundOptionsProps>) {
	const { t, i18n } = useLingui();
	const silentHintId = useId();
	const systemHintId = useId();
	const runDaemonAction = useDaemonAction();
	const isSilent = SILENT_MODES.includes(notifyMode);
	const isSystemSound = value === 'system';
	const describedBy =
		[isSilent && silentHintId, isSystemSound && systemHintId]
			.filter(Boolean)
			.join(' ') || undefined;

	const handleChange = (event: ChangeEvent<HTMLSelectElement>) => {
		const option = OPTIONS.find(
			(candidate) => candidate.value === event.target.value
		);
		if (option) onChange(option.value);
	};

	const preview = (event: SoundEventDto) =>
		void runDaemonAction(
			(gateway) => gateway.previewSound(value, event, volume),
			t`Couldn't play the sound`
		);
	const handlePreviewWoke = () => preview('woke');
	const handlePreviewWakeFailed = () => preview('wakeFailed');

	return (
		<div className="flex flex-col gap-1">
			<Select
				name="standbySound"
				label={t`Sound`}
				options={translateOptions(OPTIONS, i18n)}
				value={value}
				onChange={handleChange}
				aria-describedby={describedBy}
			/>
			{isSilent && (
				<p id={silentHintId} className="m-0 text-sm text-tn-muted">
					{t`Not played with the chosen signal.`}
				</p>
			)}
			{isSystemSound ? (
				<p id={systemHintId} className="m-0 text-sm text-tn-muted">
					{t`The system sound follows the system volume.`}
				</p>
			) : (
				<StandbyVolumeSlider
					value={volume}
					isDisabled={isSilent}
					describedBy={isSilent ? silentHintId : undefined}
					onChange={onVolumeChange}
				/>
			)}
			<div className="flex gap-2">
				<Button
					type="button"
					variant="outline"
					color="neutral"
					onClick={handlePreviewWoke}
				>
					{t`Play the wake sound`}
				</Button>
				<Button
					type="button"
					variant="outline"
					color="neutral"
					onClick={handlePreviewWakeFailed}
				>
					{t`Play the failure sound`}
				</Button>
			</div>
		</div>
	);
}
