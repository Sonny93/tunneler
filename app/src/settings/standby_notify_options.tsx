import { msg } from '@lingui/core/macro';
import { Select } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';
import { useId, type ChangeEvent } from 'react';
import type { MessageDescriptor } from '@lingui/core';

import type { StandbyNotifyDto } from '#/daemon/protocol';
import { translateOptions } from '#/i18n/translate_options';

const OPTIONS: readonly {
	value: StandbyNotifyDto;
	label: MessageDescriptor;
}[] = [
	{ value: 'none', label: msg`None` },
	{ value: 'visual', label: msg`Notification` },
	{ value: 'sound', label: msg`Sound` },
	{ value: 'both', label: msg`Both` },
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
	const { t, i18n } = useLingui();
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
				label={t`Signal when leaving standby`}
				options={translateOptions(OPTIONS, i18n)}
				value={value}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<p id={hintId} className="m-0 text-sm text-tn-muted">
				{t`When a tunnel wakes up, and when waking fails.`}
			</p>
		</div>
	);
}
