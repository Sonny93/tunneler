import { cn } from '#/shared/cn';
import type { TunnelDto } from '#/daemon/protocol';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

type TunnelEndpointProps = {
	tunnel: TunnelDto;
	className?: string;
};

/** Arrow for the eye, "vers" for assistive tech, so the arrow is never read. */
export const TunnelEndpoint = ({
	tunnel,
	className = 'text-sm text-tn-muted',
}: Readonly<TunnelEndpointProps>) => (
	<span className={cn('font-mono', className)}>
		<span aria-hidden="true">{getTunnelEndpointLabel(tunnel)}</span>
		<span className="sr-only">{getTunnelAccessibleEndpointLabel(tunnel)}</span>
	</span>
);
