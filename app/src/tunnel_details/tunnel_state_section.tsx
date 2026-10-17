import { useId } from 'react';

import type { TunnelDto } from '#/daemon/protocol';
import { TunnelStatusBadge } from '#/shared/status_badge';
import { TargetHealthBadge } from '#/shared/health_badge';
import { TunnelEndpoint } from '#/tunnels/list/tunnel_endpoint';

type TunnelStateSectionProps = {
	tunnel: TunnelDto;
};

export function TunnelStateSection({
	tunnel,
}: Readonly<TunnelStateSectionProps>) {
	const headingId = useId();

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				État
			</h3>
			<div className="flex flex-wrap gap-1">
				<TunnelStatusBadge state={tunnel.state} />
				<TargetHealthBadge health={tunnel.health} />
			</div>
			<TunnelEndpoint tunnel={tunnel} />
		</section>
	);
}
