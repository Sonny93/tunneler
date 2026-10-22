import { useLingui } from '@lingui/react/macro';
import { Button, Input } from '@minimalstuff/ui';
import { useState, type ChangeEvent } from 'react';

import { copyTextWithToast } from '#/shared/copy_text';
import type { ResolvedHostDto } from '#/daemon/protocol';
import { filterOptions } from '#/host_config/filter_options';
import { HostConfigTable } from '#/host_config/host_config_table';
import { formatConfigLines } from '#/host_config/format_config_lines';
import { HostConfigWarnings } from '#/host_config/host_config_warnings';
import { describeOptionCount } from '#/host_config/describe_option_count';

type HostConfigViewProps = {
	resolvedHost: ResolvedHostDto;
};

export function HostConfigView({
	resolvedHost,
}: Readonly<HostConfigViewProps>) {
	const { t, i18n } = useLingui();
	const [query, setQuery] = useState('');
	const shownOptions = filterOptions(resolvedHost.options, query);

	const handleQueryChange = (event: ChangeEvent<HTMLInputElement>) =>
		setQuery(event.target.value);
	const handleCopy = () =>
		void copyTextWithToast(
			formatConfigLines(resolvedHost.options),
			t`Configuration copied`
		);

	return (
		<>
			<HostConfigWarnings warnings={resolvedHost.warnings} />
			<Input
				label={t`Filter options`}
				value={query}
				onChange={handleQueryChange}
				autoComplete="off"
			/>
			<output className="block text-xs text-tn-muted">
				{describeOptionCount(shownOptions.length, i18n)}
			</output>
			<HostConfigTable host={resolvedHost.host} options={shownOptions} />
			<div>
				<Button
					startIcon="i-mdi-content-copy"
					variant="outline"
					color="neutral"
					onClick={handleCopy}
				>
					{t`Copy the configuration`}
				</Button>
			</div>
		</>
	);
}
