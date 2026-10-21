import { useId } from 'react';
import { RadioOptions } from '@minimalstuff/ui';

import { useDaemonStore } from '#/state/daemon_store';
import type { StandbyChoice } from '#/tunnel_form/tunnel_form_values';

const INHERITED_VALUE = '';

type StandbyFieldProps = {
	value: StandbyChoice;
	onChange: (choice: StandbyChoice) => void;
};

/** Whether a local or dynamic tunnel waits without an SSH session until a client connects. */
export function StandbyField({ value, onChange }: Readonly<StandbyFieldProps>) {
	const hintId = useId();
	const isStandbyByDefault = useDaemonStore(
		(state) => state.settings?.standby ?? false
	);
	const options = [
		{
			value: INHERITED_VALUE,
			label: `Réglage général (${isStandbyByDefault ? 'oui' : 'non'})`,
		},
		{ value: 'yes', label: 'Oui' },
		{ value: 'no', label: 'Non' },
	];

	const handleChange = (selected: string) =>
		onChange(selected as StandbyChoice);

	return (
		<div className="flex flex-col gap-1">
			<RadioOptions
				name="standby"
				label="Veille"
				orientation="horizontal"
				options={options}
				value={value}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<p id={hintId} className="m-0 text-sm text-tn-muted">
				Le tunnel garde son port sans session SSH jusqu&apos;à la première
				connexion. Prend effet au prochain démarrage.
			</p>
		</div>
	);
}
