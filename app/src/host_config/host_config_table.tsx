import type { ResolvedHostOptionDto } from '#/daemon/protocol';

type HostConfigTableProps = {
	host: string;
	options: readonly ResolvedHostOptionDto[];
};

/** Option names and values come from ssh, hence `lang="en"`; long values wrap. */
export function HostConfigTable({
	host,
	options,
}: Readonly<HostConfigTableProps>) {
	if (options.length === 0) return null;

	return (
		<table className="w-full table-fixed border-collapse text-left text-sm">
			<caption className="sr-only">Options effectives de {host}</caption>
			<thead>
				<tr className="border-b border-tn-divider">
					<th scope="col" className="w-2/5 py-1 pr-2 font-semibold">
						Option
					</th>
					<th scope="col" className="py-1 font-semibold">
						Valeur
					</th>
				</tr>
			</thead>
			<tbody lang="en">
				{options.map(({ key, value }, index) => (
					<tr
						key={`${key}-${index}`}
						className="border-b border-tn-divider align-top"
					>
						<th
							scope="row"
							className="break-words py-1 pr-2 font-mono font-normal"
						>
							{key}
						</th>
						<td className="break-all py-1 font-mono">{value}</td>
					</tr>
				))}
			</tbody>
		</table>
	);
}
