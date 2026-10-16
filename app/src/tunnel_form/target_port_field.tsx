import { useId, type ChangeEvent } from 'react';
import { Button, Input } from '@minimalstuff/ui';

import { FormHint } from '#/shared/form_hint';

type TargetPortFieldProps = {
	value: string;
	isLinked: boolean;
	error?: string;
	onChange: (value: string) => void;
	onRelink: () => void;
};

export function TargetPortField({
	value,
	isLinked,
	error,
	onChange,
	onRelink,
}: Readonly<TargetPortFieldProps>) {
	const hintId = useId();

	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(event.target.value);

	return (
		<div>
			<Input
				name="targetPort"
				label="Port cible"
				value={value}
				onChange={handleChange}
				error={error}
				required
				inputMode="numeric"
				autoComplete="off"
				aria-describedby={isLinked ? hintId : undefined}
			/>
			{isLinked ? (
				<FormHint id={hintId}>Suit le port d'écoute</FormHint>
			) : (
				<Button variant="ghost" size="xs" type="button" onClick={onRelink}>
					Lier au port d'écoute
				</Button>
			)}
		</div>
	);
}
