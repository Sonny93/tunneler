import { useLingui } from '@lingui/react/macro';
import { RadioOptions } from '@minimalstuff/ui';
import { useId } from 'react';

import type { X11ServerStartDto } from '#/daemon/protocol';
import { getX11ServerStartOptions } from '#/shared/x11_server_start_options';

type SettingsX11ServerStartProps = {
	value: X11ServerStartDto;
	onChange: (value: X11ServerStartDto) => void;
};

export function SettingsX11ServerStart({
	value,
	onChange,
}: Readonly<SettingsX11ServerStartProps>) {
	const { t, i18n } = useLingui();
	const hintId = useId();
	const options = getX11ServerStartOptions(i18n);
	const handleChange = (selected: string) => {
		const option = options.find((candidate) => candidate.value === selected);
		if (option) onChange(option.value);
	};

	return (
		<div className="flex flex-col gap-1">
			<RadioOptions
				name="x11ServerStart"
				label={t`When no X server is running`}
				options={options}
				value={value}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<p id={hintId} className="m-0 text-sm text-tn-muted">
				{t`Tunneler can start VcXsrv or Xming when it is installed (Windows).`}
			</p>
		</div>
	);
}
