import type { ChangeEvent } from 'react';
import { Button, Input } from '@minimalstuff/ui';

const LINKED_HINT = "Suit le port d'écoute";

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
				description={isLinked ? LINKED_HINT : undefined}
			/>
			{!isLinked && (
				<Button variant="ghost" size="xs" type="button" onClick={onRelink}>
					Lier au port d'écoute
				</Button>
			)}
		</div>
	);
}
