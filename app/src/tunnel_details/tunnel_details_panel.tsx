import { useLingui } from '@lingui/react/macro';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { TunnelLogSection } from '#/tunnel_details/tunnel_log_section';
import { TunnelStateSection } from '#/tunnel_details/tunnel_state_section';
import { TunnelActionsSection } from '#/tunnel_details/tunnel_actions_section';
import { TunnelMetricsSection } from '#/tunnel_details/tunnel_metrics_section';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

type TunnelDetailsPanelProps = {
	tunnel: TunnelDto;
};

export function TunnelDetailsPanel({
	tunnel,
}: Readonly<TunnelDetailsPanelProps>) {
	const { t, i18n } = useLingui();
	const closePanel = useUiStore((state) => state.closePanel);

	const endpointLabel = getTunnelEndpointLabel(tunnel);

	return (
		<Drawer isOpen title={t`Tunnel ${endpointLabel}`} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<TunnelStateSection tunnel={tunnel} />
				<TunnelMetricsSection metrics={tunnel.metrics} />
				<TunnelActionsSection tunnel={tunnel} />
				<TunnelLogSection
					tunnelId={tunnel.id}
					tunnelLabel={getTunnelAccessibleEndpointLabel(tunnel, i18n)}
				/>
			</div>
		</Drawer>
	);
}
