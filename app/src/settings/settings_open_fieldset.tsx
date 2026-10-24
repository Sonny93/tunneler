import { useLingui } from '@lingui/react/macro';
import { RadioOptions } from '@minimalstuff/ui';

import type { OpenDefaultDto, OpenHostDto } from '#/daemon/protocol';
import { getOpenDefaultOptions } from '#/shared/open_default_options';
import { getOpenHostOptions } from '#/shared/open_host_options';

type SettingsOpenFieldsetProps = {
	host: OpenHostDto;
	onHostChange: (host: OpenHostDto) => void;
	openDefault: OpenDefaultDto;
	onOpenDefaultChange: (openDefault: OpenDefaultDto) => void;
};

export function SettingsOpenFieldset({
	host,
	onHostChange,
	openDefault,
	onOpenDefaultChange,
}: Readonly<SettingsOpenFieldsetProps>) {
	const { t, i18n } = useLingui();
	const hostOptions = getOpenHostOptions(i18n);
	const openDefaultOptions = getOpenDefaultOptions(i18n);
	const handleHostChange = (selected: string) => {
		const option = hostOptions.find(
			(candidate) => candidate.value === selected
		);
		if (option) onHostChange(option.value);
	};
	const handleOpenDefaultChange = (selected: string) => {
		const option = openDefaultOptions.find(
			(candidate) => candidate.value === selected
		);
		if (option) onOpenDefaultChange(option.value);
	};

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Opening`}</legend>
			<RadioOptions
				name="openHost"
				label={t`Host to open`}
				options={hostOptions}
				value={host}
				onChange={handleHostChange}
			/>
			<RadioOptions
				name="openDefault"
				label={t`Tunnels without an application`}
				options={openDefaultOptions}
				value={openDefault}
				onChange={handleOpenDefaultChange}
			/>
		</fieldset>
	);
}
