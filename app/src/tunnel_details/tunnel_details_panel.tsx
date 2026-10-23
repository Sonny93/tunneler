import { useLingui } from '@lingui/react/macro';
import { Drawer } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { useUiStore } from '#/state/ui_store';
import { TunnelActionsSection } from '#/tunnel_details/tunnel_actions_section';
import { TunnelLogSection } from '#/tunnel_details/tunnel_log_section';
import { TunnelMetricsSection } from '#/tunnel_details/tunnel_metrics_section';
import { TunnelStateSection } from '#/tunnel_details/tunnel_state_section';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

type TunnelDetailsPanelProps = {
	tunnel: TunnelDto;
	isOpen: boolean;
};

export function TunnelDetailsPanel({
	tunnel,
	isOpen,
}: Readonly<TunnelDetailsPanelProps>) {
	const { t, i18n } = useLingui();
	const closePanel = useUiStore((state) => state.closePanel);

	const endpointLabel = getTunnelEndpointLabel(tunnel);

	return (
		<Drawer
			isOpen={isOpen}
			size="lg"
			title={t`Tunnel ${endpointLabel}`}
			onClose={closePanel}
			className="flex flex-col"
		>
			<div className="flex flex-1 flex-col gap-4">
				<TunnelStateSection tunnel={tunnel} />
				<TunnelActionsSection tunnel={tunnel} />
				<TunnelMetricsSection metrics={tunnel.metrics} />
				<TunnelLogSection
					tunnelId={tunnel.id}
					tunnelLabel={getTunnelAccessibleEndpointLabel(tunnel, i18n)}
				/>
			</div>
		</Drawer>
	);
}
