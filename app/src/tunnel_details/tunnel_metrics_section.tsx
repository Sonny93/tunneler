import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { Card } from '@minimalstuff/ui';

import type { MetricsDto } from '#/daemon/protocol';
import { formatBytes } from '#/shared/format_bytes';
import { formatTime } from '#/shared/format_time';

type TunnelMetricsSectionProps = {
	metrics: MetricsDto;
};

type MetricRow = { label: string; value: string };

function buildMetricRows(
	metrics: Readonly<MetricsDto>,
	i18n: I18n
): MetricRow[] {
	return [
		{
			label: i18n._(msg`Active connections`),
			value: String(metrics.activeConnections),
		},
		{
			label: i18n._(msg`Total connections`),
			value: String(metrics.totalConnections),
		},
		{ label: i18n._(msg`Received`), value: formatBytes(metrics.bytesIn) },
		{ label: i18n._(msg`Sent`), value: formatBytes(metrics.bytesOut) },
		{
			label: i18n._(msg`Last activity`),
			value:
				metrics.lastActivity === undefined
					? i18n._(msg`None`)
					: formatTime(metrics.lastActivity),
		},
	];
}

export function TunnelMetricsSection({
	metrics,
}: Readonly<TunnelMetricsSectionProps>) {
	const { i18n } = useLingui();

	return (
		<Card title={i18n._(msg`Metrics`)} headingLevel={3} padding="sm">
			<dl className="m-0 grid grid-cols-2 gap-2 sm:grid-cols-3">
				{buildMetricRows(metrics, i18n).map(({ label, value }) => (
					<div
						key={label}
						className="flex min-w-0 flex-col gap-1 rounded-md border border-tn-divider bg-tn-surface p-3"
					>
						<dt className="text-xs text-tn-muted">{label}</dt>
						<dd className="m-0 break-words font-mono text-xl font-semibold text-tn-text">
							{value}
						</dd>
					</div>
				))}
			</dl>
		</Card>
	);
}
