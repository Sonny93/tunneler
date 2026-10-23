import { useLingui } from '@lingui/react/macro';
import { Card } from '@minimalstuff/ui';

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

	return (
		<Card title={t`State`} headingLevel={3} padding="sm">
			<div className="flex flex-col gap-2">
				<div className="flex flex-wrap gap-1">
					{tunnel.isTemporary && <TemporaryBadge />}
					<TunnelStatusBadge state={tunnel.state} />
					<TargetHealthBadge health={tunnel.health} />
				</div>
				<TunnelEndpoint tunnel={tunnel} />
			</div>
		</Card>
	);
}
