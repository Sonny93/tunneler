import { useLingui } from '@lingui/react/macro';
import { RadioOptions } from '@minimalstuff/ui';

import type { TunnelKindDto } from '#/daemon/protocol';
import { TUNNEL_KIND_OPTIONS } from '#/tunnel_form/tunnel_kind_texts';

type TunnelKindFieldProps = {
	value: TunnelKindDto;
	onChange: (kind: TunnelKindDto) => void;
};

export function TunnelKindField({
	value,
	onChange,
}: Readonly<TunnelKindFieldProps>) {
	const { t, i18n } = useLingui();
	const options = TUNNEL_KIND_OPTIONS.map((option) => ({
		value: option.value,
		label: i18n._(option.label),
		description: i18n._(option.description),
	}));

	const handleChange = (selected: string) => {
		const option = options.find(({ value: kind }) => kind === selected);
		if (option) onChange(option.value);
	};

	return (
		<RadioOptions
			name="kind"
			label={t`Tunnel type`}
			options={options}
			value={value}
			onChange={handleChange}
		/>
	);
}
