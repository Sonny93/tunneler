import { useLingui } from '@lingui/react/macro';
import { useId, type ChangeEvent } from 'react';

const MINIMUM_VOLUME = 5;
const MAXIMUM_VOLUME = 100;
const VOLUME_STEP = 5;

type StandbyVolumeSliderProps = {
	value: number;
	isDisabled: boolean;
	/** The id of the element that explains why the slider is disabled. */
	describedBy?: string;
	onChange: (volume: number) => void;
};

/** The volume, in percent, of the sounds the daemon synthesizes. */
export function StandbyVolumeSlider({
	value,
	isDisabled,
	describedBy,
	onChange,
}: Readonly<StandbyVolumeSliderProps>) {
	const { t } = useLingui();
	const inputId = useId();
	const percentage = t`${value} %`;

	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(Number(event.target.value));

	return (
		<div className="flex flex-col gap-1">
			<label htmlFor={inputId} className="text-sm font-medium text-tn-text">
				{t`Volume`}
			</label>
			<div className="flex items-center gap-3">
				<input
					id={inputId}
					name="standbyVolume"
					type="range"
					min={MINIMUM_VOLUME}
					max={MAXIMUM_VOLUME}
					step={VOLUME_STEP}
					value={value}
					disabled={isDisabled}
					aria-valuetext={percentage}
					aria-describedby={describedBy}
					onChange={handleChange}
					className="flex-1 accent-tn-accent"
				/>
				<output htmlFor={inputId} className="w-12 text-end tabular-nums">
					{percentage}
				</output>
			</div>
		</div>
	);
}
