import { useLingui } from '@lingui/react/macro';
import { RadioOptions } from '@minimalstuff/ui';
import { useId } from 'react';

import { getRadioOrientation } from '#/shared/radio_orientation';
import { useDaemonStore } from '#/state/daemon_store';
import type { StandbyChoice } from '#/tunnel_form/tunnel_form_values';

const INHERITED_VALUE = '';

type StandbyFieldProps = {
	value: StandbyChoice;
	onChange: (choice: StandbyChoice) => void;
};

/** Whether a local or dynamic tunnel waits without an SSH session until a client connects. */
export function StandbyField({ value, onChange }: Readonly<StandbyFieldProps>) {
	const { t } = useLingui();
	const hintId = useId();
	const isStandbyByDefault = useDaemonStore(
		(state) => state.settings?.standby ?? false
	);
	const options = [
		{
			value: INHERITED_VALUE,
			label: isStandbyByDefault
				? t`General setting (yes)`
				: t`General setting (no)`,
		},
		{ value: 'yes', label: t`Yes` },
		{ value: 'no', label: t`No` },
	];

	const handleChange = (selected: string) =>
		onChange(selected as StandbyChoice);

	return (
		<div className="flex flex-col gap-1">
			<RadioOptions
				name="standby"
				label={t`Wait in standby`}
				orientation={getRadioOrientation(options.length)}
				options={options}
				value={value}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<p id={hintId} className="m-0 text-sm text-tn-muted">
				{t`The tunnel keeps its port without an SSH session until the first connection. Takes effect on the next start.`}
			</p>
		</div>
	);
}
