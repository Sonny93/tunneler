import { useId } from 'react';
import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { TunnelLogList } from '#/tunnel_details/tunnel_log_list';
import { useTunnelLogs } from '#/tunnel_details/use_tunnel_logs';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

type TunnelLogSectionProps = {
	tunnelId: string;
	tunnelLabel: string;
};

export function TunnelLogSection({
	tunnelId,
	tunnelLabel,
}: Readonly<TunnelLogSectionProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const { state, retry } = useTunnelLogs(tunnelId);
	const errorMessage =
		state.error === null ? '' : describeDaemonError(state.error, i18n);

	return (
		<section
			aria-labelledby={headingId}
			aria-busy={state.status === 'loading'}
			className="flex flex-col gap-2"
		>
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{t`Log`}
			</h3>
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
		</section>
	);
}
