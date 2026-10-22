import { useId } from 'react';
import { Button, RadioOptions } from '@minimalstuff/ui';

import { useDaemonAction } from '#/daemon/use_daemon_action';
import { getRadioOrientation } from '#/shared/radio_orientation';
import type {
	SoundEventDto,
	StandbyNotifyDto,
	StandbySoundDto,
} from '#/daemon/protocol';

const OPTIONS: readonly { value: StandbySoundDto; label: string }[] = [
	{ value: 'pluck', label: 'Corde pincée' },
	{ value: 'chime', label: 'Carillon' },
	{ value: 'marimba', label: 'Marimba' },
	{ value: 'system', label: 'Son du système' },
];

const SILENT_MODES: readonly StandbyNotifyDto[] = ['none', 'visual'];

type StandbySoundOptionsProps = {
	value: StandbySoundDto;
	notifyMode: StandbyNotifyDto;
	onChange: (sound: StandbySoundDto) => void;
};

/** Which sound signals a wake or a failed wake, with a preview of both. */
export function StandbySoundOptions({
	value,
	notifyMode,
	onChange,
}: Readonly<StandbySoundOptionsProps>) {
	const systemHintId = useId();
	const silentHintId = useId();
	const runDaemonAction = useDaemonAction();
	const isSilent = SILENT_MODES.includes(notifyMode);
	const describedBy = isSilent
		? `${systemHintId} ${silentHintId}`
		: systemHintId;

	const handleChange = (selected: string) => {
		const option = OPTIONS.find((candidate) => candidate.value === selected);
		if (option) onChange(option.value);
	};

	const preview = (event: SoundEventDto) =>
		void runDaemonAction(
			(gateway) => gateway.previewSound(value, event),
			'Écoute impossible'
		);
	const handlePreviewWoke = () => preview('woke');
	const handlePreviewWakeFailed = () => preview('wakeFailed');

	return (
		<div className="flex flex-col gap-1">
			<RadioOptions
				name="standbySound"
				label="Son"
				orientation={getRadioOrientation(OPTIONS.length)}
				options={OPTIONS.map((option) => ({ ...option }))}
				value={value}
				onChange={handleChange}
				aria-describedby={describedBy}
			/>
			<p id={systemHintId} className="m-0 text-sm text-tn-muted">
				Son du système : suit le schéma de sons de Windows ou le thème de sons
				du bureau.
			</p>
			{isSilent && (
				<p id={silentHintId} className="m-0 text-sm text-tn-muted">
					Choisissez « Son » ou « Les deux » pour l'entendre au réveil.
				</p>
			)}
			<div className="flex gap-2">
				<Button
					type="button"
					variant="outline"
					color="neutral"
					onClick={handlePreviewWoke}
				>
					Écouter le réveil
				</Button>
				<Button
					type="button"
					variant="outline"
					color="neutral"
					onClick={handlePreviewWakeFailed}
				>
					Écouter l'échec
				</Button>
			</div>
		</div>
	);
}
