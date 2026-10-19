import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type TunnelLabelListProps = {
	tunnels: readonly TunnelDto[];
};

export function TunnelLabelList({ tunnels }: Readonly<TunnelLabelListProps>) {
	return (
		<ul className="m-0 list-disc pl-5">
			{tunnels.map((tunnel) => (
				<li key={tunnel.id}>{getTunnelEndpointLabel(tunnel)}</li>
			))}
		</ul>
	);
}
