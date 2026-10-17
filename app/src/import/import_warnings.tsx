import { useId } from 'react';

import type { ImportWarningDto } from '#/daemon/protocol';
import { describeImportWarning } from '#/import/describe_import_warning';

type ImportWarningsProps = {
	warnings: readonly ImportWarningDto[];
};

export function ImportWarnings({ warnings }: Readonly<ImportWarningsProps>) {
	const headingId = useId();
	if (warnings.length === 0) return null;

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-1">
			<h4 id={headingId} className="m-0 text-sm font-semibold">
				Avertissements
			</h4>
			<ul className="m-0 flex list-disc flex-col gap-1 pl-5 text-sm">
				{warnings.map((warning) => {
					const { before, raw, after } = describeImportWarning(warning);
					return (
						<li key={`${before}${raw}${after}`}>
							{before}
							<code lang="en" className="break-all">
								{raw}
							</code>
							{after}
						</li>
					);
				})}
			</ul>
		</section>
	);
}
