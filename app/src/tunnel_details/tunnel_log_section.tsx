import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

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
	const headingId = useId();
	const { state, retry } = useTunnelLogs(tunnelId);

	return (
		<section
			aria-labelledby={headingId}
			aria-busy={state.status === 'loading'}
			className="flex flex-col gap-2"
		>
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				Journal
			</h3>
			{state.status === 'loading' && (
				<p className="m-0 text-sm text-tn-muted">Chargement du journal…</p>
			)}
			{state.status === 'error' && (
				<div role="alert" className="flex flex-col items-start gap-2 text-sm">
					<p className="m-0 text-tn-danger">
						Journal indisponible : {state.errorMessage}
					</p>
					<Button
						startIcon="i-mdi-refresh"
						variant="outline"
						color="neutral"
						onClick={retry}
					>
						Réessayer
					</Button>
				</div>
			)}
			{state.status === 'ready' && (
				<TunnelLogList entries={state.entries} tunnelLabel={tunnelLabel} />
			)}
		</section>
	);
}
