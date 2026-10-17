import { useId } from 'react';

import { formatTime } from '#/shared/format_time';
import type { MetricsDto } from '#/daemon/protocol';
import { formatBytes } from '#/shared/format_bytes';

const NO_ACTIVITY_TEXT = 'Aucune';

type TunnelMetricsSectionProps = {
	metrics: MetricsDto;
};

type MetricRow = { label: string; value: string };

function buildMetricRows(metrics: Readonly<MetricsDto>): MetricRow[] {
	return [
		{
			label: 'Connexions actives',
			value: String(metrics.activeConnections),
		},
		{
			label: 'Connexions totales',
			value: String(metrics.totalConnections),
		},
		{ label: 'Reçu', value: formatBytes(metrics.bytesIn) },
		{ label: 'Envoyé', value: formatBytes(metrics.bytesOut) },
		{
			label: 'Dernière activité',
			value:
				metrics.lastActivity === undefined
					? NO_ACTIVITY_TEXT
					: formatTime(metrics.lastActivity),
		},
	];
}

export function TunnelMetricsSection({
	metrics,
}: Readonly<TunnelMetricsSectionProps>) {
	const headingId = useId();

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				Métriques
			</h3>
			<dl className="m-0 flex flex-col gap-1 text-sm">
				{buildMetricRows(metrics).map(({ label, value }) => (
					<div key={label} className="flex justify-between gap-4">
						<dt className="text-tn-muted">{label}</dt>
						<dd className="m-0 font-mono">{value}</dd>
					</div>
				))}
			</dl>
		</section>
	);
}
