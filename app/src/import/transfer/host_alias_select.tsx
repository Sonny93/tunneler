import { useLingui } from '@lingui/react/macro';
import { Select } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import { NO_HOST_ALIAS } from '#/import/transfer/config_import_plan';

type HostAliasSelectProps = {
	fileHost: string;
	alias: string;
	aliases: readonly string[];
	isBusy: boolean;
	onChange: (fileHost: string, alias: string) => void;
};

/** Lets the user replace a host of the file, unknown here, by a local alias. */
export function HostAliasSelect({
	fileHost,
	alias,
	aliases,
	isBusy,
	onChange,
}: Readonly<HostAliasSelectProps>) {
	const { t } = useLingui();
	const options = [
		{ value: NO_HOST_ALIAS, label: t`Keep ${fileHost}` },
		...aliases.map((known) => ({ value: known, label: known })),
	];

	const handleChange = (event: ChangeEvent<HTMLSelectElement>) =>
		onChange(fileHost, event.target.value);

	return (
		<Select
			name={`alias-${fileHost}`}
			label={t`Use this alias for ${fileHost}`}
			description={
				alias === NO_HOST_ALIAS
					? t`No ssh_config Host entry names it exactly.`
					: undefined
			}
			options={options}
			value={alias}
			onChange={handleChange}
			aria-busy={isBusy}
		/>
	);
}
