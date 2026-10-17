import { useId } from 'react';

type HostConfigWarningsProps = {
	warnings: readonly string[];
};

/** Warnings come from the daemon in English, hence `lang="en"`. */
export function HostConfigWarnings({
	warnings,
}: Readonly<HostConfigWarningsProps>) {
	const headingId = useId();
	if (warnings.length === 0) return null;

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-1">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				Avertissements
			</h3>
			<ul lang="en" className="m-0 flex list-disc flex-col gap-1 pl-5 text-sm">
				{warnings.map((warning) => (
					<li key={warning}>{warning}</li>
				))}
			</ul>
		</section>
	);
}
