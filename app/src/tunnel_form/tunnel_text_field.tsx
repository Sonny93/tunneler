import { Input } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import { FormHint } from '#/shared/form_hint';
import type { TunnelFormTextField } from '#/tunnel_form/tunnel_form_reducer';

type TunnelTextFieldProps = {
	field: TunnelFormTextField;
	label: string;
	value: string;
	onChange: (field: TunnelFormTextField, value: string) => void;
	error?: string;
	hint?: string;
	placeholder?: string;
	isRequired?: boolean;
	inputMode?: 'numeric';
	listId?: string;
};

export function TunnelTextField({
	field,
	label,
	value,
	onChange,
	error,
	hint,
	placeholder,
	isRequired = false,
	inputMode,
	listId,
}: Readonly<TunnelTextFieldProps>) {
	const hintId = useId();

	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(field, event.target.value);

	return (
		<div>
			<Input
				name={field}
				label={label}
				value={value}
				onChange={handleChange}
				error={error}
				required={isRequired}
				placeholder={placeholder}
				inputMode={inputMode}
				list={listId}
				autoComplete="off"
				aria-describedby={hint ? hintId : undefined}
			/>
			{hint && <FormHint id={hintId}>{hint}</FormHint>}
		</div>
	);
}
