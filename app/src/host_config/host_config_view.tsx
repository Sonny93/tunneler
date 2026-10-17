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
	const [query, setQuery] = useState('');
	const shownOptions = filterOptions(resolvedHost.options, query);

	const handleQueryChange = (event: ChangeEvent<HTMLInputElement>) =>
		setQuery(event.target.value);
	const handleCopy = () =>
		void copyTextWithToast(
			formatConfigLines(resolvedHost.options),
			'Configuration copiée'
		);

	return (
		<>
			<HostConfigWarnings warnings={resolvedHost.warnings} />
			<Input
				label="Filtrer les options"
				value={query}
				onChange={handleQueryChange}
				autoComplete="off"
			/>
			<output className="block text-xs text-tn-muted">
				{describeOptionCount(shownOptions.length)}
			</output>
			<HostConfigTable host={resolvedHost.host} options={shownOptions} />
			<div>
				<Button
					startIcon="i-mdi-content-copy"
					variant="outline"
					color="neutral"
					onClick={handleCopy}
				>
					Copier la configuration
				</Button>
			</div>
		</>
	);
}
