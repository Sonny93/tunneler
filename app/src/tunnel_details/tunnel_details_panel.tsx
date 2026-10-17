import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { TunnelLogSection } from '#/tunnel_details/tunnel_log_section';
import { TunnelStateSection } from '#/tunnel_details/tunnel_state_section';
import { TunnelActionsSection } from '#/tunnel_details/tunnel_actions_section';
import { TunnelMetricsSection } from '#/tunnel_details/tunnel_metrics_section';

type TunnelDetailsPanelProps = {
	tunnel: TunnelDto;
};

export function TunnelDetailsPanel({
	tunnel,
}: Readonly<TunnelDetailsPanelProps>) {
	const closePanel = useUiStore((state) => state.closePanel);

	return (
		<Drawer isOpen title={`Tunnel ${tunnel.name}`} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<TunnelStateSection tunnel={tunnel} />
				<TunnelMetricsSection metrics={tunnel.metrics} />
				<TunnelActionsSection tunnel={tunnel} />
				<TunnelLogSection tunnelId={tunnel.id} tunnelName={tunnel.name} />
			</div>
		</Drawer>
	);
}
