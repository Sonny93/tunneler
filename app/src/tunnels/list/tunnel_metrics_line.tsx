import type { MetricsDto } from '#/daemon/protocol';
import { formatBytes } from '#/shared/format_bytes';

type TunnelMetricsLineProps = {
	metrics: MetricsDto;
};

function describeConnections(count: number): string {
	return count > 1 ? `${count} connexions` : `${count} connexion`;
}

export function TunnelMetricsLine({
	metrics,
}: Readonly<TunnelMetricsLineProps>) {
	const connections = describeConnections(metrics.activeConnections);
	const received = formatBytes(metrics.bytesIn);
	const sent = formatBytes(metrics.bytesOut);

	return (
		<p className="m-0 text-xs text-tn-muted">
			<span aria-hidden="true">
				{connections} · ↓ {received} ↑ {sent}
			</span>
			<span className="sr-only">
				{connections}, {received} reçus, {sent} envoyés
			</span>
		</p>
	);
}
