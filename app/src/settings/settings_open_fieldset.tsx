import { RadioOptions } from '@minimalstuff/ui';

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
	const handleChange = (selected: string) => {
		const option = OPTIONS.find((candidate) => candidate.value === selected);
		if (option) onChange(option.value);
	};

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">Ouverture</legend>
			<RadioOptions
				name="openHost"
				label="Hôte à ouvrir"
				options={OPTIONS}
				value={value}
				onChange={handleChange}
			/>
		</fieldset>
	);
}
