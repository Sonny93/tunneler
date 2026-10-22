import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';

import type { MetricsDto } from '#/daemon/protocol';
import { formatBytes } from '#/shared/format_bytes';

type TunnelMetricsLineProps = {
	metrics: MetricsDto;
};

export function TunnelMetricsLine({
	metrics,
}: Readonly<TunnelMetricsLineProps>) {
	const { t } = useLingui();
	const connectionCount = metrics.activeConnections;
	const connections = t`${plural(connectionCount, {
		one: '# connection',
		other: '# connections',
	})}`;
	const received = formatBytes(metrics.bytesIn);
	const sent = formatBytes(metrics.bytesOut);
	const spokenMetrics = t`${connections}, ${received} received, ${sent} sent`;

	return (
		<p className="m-0 text-xs text-tn-muted">
			<span aria-hidden="true">
				{connections} · ↓ {received} ↑ {sent}
			</span>
			<span className="sr-only">{spokenMetrics}</span>
		</p>
	);
}
