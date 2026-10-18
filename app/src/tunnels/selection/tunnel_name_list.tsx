import type { TunnelDto } from '#/daemon/protocol';

type TunnelNameListProps = {
	tunnels: readonly TunnelDto[];
};

export function TunnelNameList({ tunnels }: Readonly<TunnelNameListProps>) {
	return (
		<ul className="m-0 list-disc pl-5">
			{tunnels.map(({ id, name }) => (
				<li key={id}>{name}</li>
			))}
		</ul>
	);
}
