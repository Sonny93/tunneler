import { RadioOptions } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import type { OpenHostDto } from '#/daemon/protocol';
import { OPEN_HOST_OPTIONS } from '#/shared/open_host_options';

const OPTIONS = OPEN_HOST_OPTIONS.map((option) => ({ ...option }));

type SettingsOpenFieldsetProps = {
	value: OpenHostDto;
	onChange: (host: OpenHostDto) => void;
};

export function SettingsOpenFieldset({
	value,
	onChange,
}: Readonly<SettingsOpenFieldsetProps>) {
	const { t } = useLingui();
	const handleChange = (selected: string) => {
		const option = OPTIONS.find((candidate) => candidate.value === selected);
		if (option) onChange(option.value);
	};

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Opening`}</legend>
			<RadioOptions
				name="openHost"
				label={t`Host to open`}
				options={OPTIONS}
				value={value}
				onChange={handleChange}
			/>
		</fieldset>
	);
}
