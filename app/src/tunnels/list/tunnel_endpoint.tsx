import type { TunnelDto } from '#/daemon/protocol';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

type TunnelEndpointProps = {
	tunnel: TunnelDto;
};

/** Arrow for the eye, "vers" for assistive tech, so the arrow is never read. */
export const TunnelEndpoint = ({ tunnel }: Readonly<TunnelEndpointProps>) => (
	<span className="font-mono text-sm text-tn-muted">
		<span aria-hidden="true">{getTunnelEndpointLabel(tunnel)}</span>
		<span className="sr-only">{getTunnelAccessibleEndpointLabel(tunnel)}</span>
	</span>
);
