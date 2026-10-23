import { useLingui } from '@lingui/react/macro';
import { useId } from 'react';

import type { TunnelDto } from '#/daemon/protocol';
import { TargetHealthBadge } from '#/shared/health_badge';
import { TunnelStatusBadge } from '#/shared/status_badge';
import { TemporaryBadge } from '#/shared/temporary_badge';
import { TunnelEndpoint } from '#/tunnels/list/tunnel_endpoint';

type TunnelStateSectionProps = {
	tunnel: TunnelDto;
};

export function TunnelStateSection({
	tunnel,
}: Readonly<TunnelStateSectionProps>) {
	const { t } = useLingui();
	const headingId = useId();

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{t`State`}
			</h3>
			<div className="flex flex-wrap gap-1">
				{tunnel.isTemporary && <TemporaryBadge />}
				<TunnelStatusBadge state={tunnel.state} />
				<TargetHealthBadge health={tunnel.health} />
			</div>
			<TunnelEndpoint tunnel={tunnel} />
		</section>
	);
}
