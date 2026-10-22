import type { ChangeEvent } from 'react';
import { useLingui } from '@lingui/react/macro';
import { Button, Input } from '@minimalstuff/ui';

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
	const { t } = useLingui();
	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(event.target.value);

	return (
		<div>
			<Input
				name="targetPort"
				label={t`Target port`}
				value={value}
				onChange={handleChange}
				error={error}
				required
				inputMode="numeric"
				autoComplete="off"
				description={isLinked ? t`Follows the listen port` : undefined}
			/>
			{!isLinked && (
				<Button variant="ghost" size="xs" type="button" onClick={onRelink}>
					{t`Link to the listen port`}
				</Button>
			)}
		</div>
	);
}
