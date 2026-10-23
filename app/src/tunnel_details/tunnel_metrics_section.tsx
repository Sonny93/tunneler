import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useId } from 'react';

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
			label: t(i18n)`Active connections`,
			value: String(metrics.activeConnections),
		},
		{
			label: t(i18n)`Total connections`,
			value: String(metrics.totalConnections),
		},
		{ label: t(i18n)`Received`, value: formatBytes(metrics.bytesIn) },
		{ label: t(i18n)`Sent`, value: formatBytes(metrics.bytesOut) },
		{
			label: t(i18n)`Last activity`,
			value:
				metrics.lastActivity === undefined
					? t(i18n)`None`
					: formatTime(metrics.lastActivity),
		},
	];
}

export function TunnelMetricsSection({
	metrics,
}: Readonly<TunnelMetricsSectionProps>) {
	const { i18n } = useLingui();
	const headingId = useId();

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{t(i18n)`Metrics`}
			</h3>
			<dl className="m-0 flex flex-col gap-1 text-sm">
				{buildMetricRows(metrics, i18n).map(({ label, value }) => (
					<div key={label} className="flex justify-between gap-4">
						<dt className="text-tn-muted">{label}</dt>
						<dd className="m-0 font-mono">{value}</dd>
					</div>
				))}
			</dl>
		</section>
	);
}
