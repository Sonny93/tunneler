import { Select } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import type { StandbyNotifyDto } from '#/daemon/protocol';

const OPTIONS: readonly { value: StandbyNotifyDto; label: string }[] = [
	{ value: 'none', label: 'Aucun' },
	{ value: 'visual', label: 'Notification' },
	{ value: 'sound', label: 'Son' },
	{ value: 'both', label: 'Les deux' },
];

type StandbyNotifyOptionsProps = {
	value: StandbyNotifyDto;
	onChange: (mode: StandbyNotifyDto) => void;
};

/** How the daemon signals a tunnel that leaves standby, or fails to. */
export function StandbyNotifyOptions({
	value,
	onChange,
}: Readonly<StandbyNotifyOptionsProps>) {
	const hintId = useId();

	const handleChange = (event: ChangeEvent<HTMLSelectElement>) => {
		const option = OPTIONS.find(
			(candidate) => candidate.value === event.target.value
		);
		if (option) onChange(option.value);
	};

	return (
		<div className="flex flex-col gap-1">
			<Select
				name="standbyNotify"
				label="Signal à la sortie de veille"
				options={OPTIONS.map((option) => ({ ...option }))}
				value={value}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<p id={hintId} className="m-0 text-sm text-tn-muted">
				Le son est joué par Tunneler lui-même : il reste audible même quand les
				notifications du système sont désactivées. Il signale aussi un réveil
				qui échoue.
			</p>
		</div>
	);
}
