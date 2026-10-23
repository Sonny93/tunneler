import { useLingui } from '@lingui/react/macro';
import { Button, Card } from '@minimalstuff/ui';

import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { TunnelLogList } from '#/tunnel_details/tunnel_log_list';
import { useTunnelLogs } from '#/tunnel_details/use_tunnel_logs';

type TunnelLogSectionProps = {
	tunnelId: string;
	tunnelLabel: string;
};

export function TunnelLogSection({
	tunnelId,
	tunnelLabel,
}: Readonly<TunnelLogSectionProps>) {
	const { t, i18n } = useLingui();
	const { state, retry } = useTunnelLogs(tunnelId);
	const errorMessage =
		state.error === null ? '' : describeDaemonError(state.error, i18n);

	return (
		<Card
			title={t`Log`}
			headingLevel={3}
			padding="sm"
			className="flex flex-1 flex-col"
		>
			{state.status === 'loading' && (
				<p className="m-0 text-sm text-tn-muted">{t`Loading the log…`}</p>
			)}
			{state.status === 'error' && (
				<div role="alert" className="flex flex-col items-start gap-2 text-sm">
					<p className="m-0 text-tn-danger">
						{t`Log unavailable: ${errorMessage}`}
					</p>
					<Button
						startIcon="i-mdi-refresh"
						variant="outline"
						color="neutral"
						onClick={retry}
					>
						{t`Retry`}
					</Button>
				</div>
			)}
			{state.status === 'ready' && (
				<TunnelLogList entries={state.entries} tunnelLabel={tunnelLabel} />
			)}
		</Card>
	);
}
