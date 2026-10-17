import { RadioOptions } from '@minimalstuff/ui';

import type { TunnelKindDto } from '#/daemon/protocol';
import { TUNNEL_KIND_OPTIONS } from '#/tunnel_form/tunnel_kind_texts';

type TunnelKindFieldProps = {
	value: TunnelKindDto;
	onChange: (kind: TunnelKindDto) => void;
};

const OPTIONS = TUNNEL_KIND_OPTIONS.map((option) => ({ ...option }));

export function TunnelKindField({
	value,
	onChange,
}: Readonly<TunnelKindFieldProps>) {
	const handleChange = (selected: string) => {
		const option = OPTIONS.find(({ value: kind }) => kind === selected);
		if (option) onChange(option.value);
	};

	return (
		<RadioOptions
			name="kind"
			label="Type de tunnel"
			options={OPTIONS}
			value={value}
			onChange={handleChange}
		/>
	);
}
