import { useId } from 'react';
import { Switch } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { TunnelStatusBadge } from '#/shared/status_badge';
import { TargetHealthBadge } from '#/shared/health_badge';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { TunnelEndpoint } from '#/tunnels/list/tunnel_endpoint';
import { TunnelTagChips } from '#/tunnels/list/tunnel_tag_chips';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { TunnelMetricsLine } from '#/tunnels/list/tunnel_metrics_line';
import { TunnelActionsMenu } from '#/tunnels/list/tunnel_actions_menu';
import { getTunnelAccessibleName } from '#/tunnels/tunnel_accessible_name';
import { describeTunnelProblems } from '#/tunnels/describe_tunnel_problems';
import {
	ROVING_CONTROL_ATTRIBUTE,
	ROVING_ROW_ATTRIBUTE,
} from '#/tunnels/list/use_roving_rows';

type TunnelRowProps = {
	tunnel: TunnelDto;
	isActiveRow: boolean;
};

export function TunnelRow({ tunnel, isActiveRow }: Readonly<TunnelRowProps>) {
	const { toggleRunning } = useTunnelActions(tunnel);
	const isRunning = isTunnelRunning(tunnel.state);
	const problemsId = useId();
	const problems = describeTunnelProblems(tunnel);

	return (
		<li
			{...{ [ROVING_ROW_ATTRIBUTE]: tunnel.id }}
			className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-md border border-tn-border bg-tn-surface px-3 py-2"
		>
			<Switch
				aria-label={getTunnelAccessibleName(tunnel)}
				aria-describedby={problems === null ? undefined : problemsId}
				checked={isRunning}
				onChange={toggleRunning}
				tabIndex={isActiveRow ? 0 : -1}
				wrapperClassName="shrink-0"
				{...{ [ROVING_CONTROL_ATTRIBUTE]: 'switch' }}
			/>
			{problems !== null && (
				<span id={problemsId} aria-hidden="true" className="sr-only">
					{problems}
				</span>
			)}
			<div className="flex min-w-40 flex-1 flex-col gap-1">
				<span className="font-semibold text-tn-text">{tunnel.name}</span>
				<TunnelEndpoint tunnel={tunnel} />
				<TunnelTagChips tags={tunnel.tags} />
			</div>
			<div className="flex min-w-0 flex-1 basis-56 flex-col items-start gap-1">
				<div className="flex flex-wrap gap-1">
					<TunnelStatusBadge state={tunnel.state} />
					<TargetHealthBadge health={tunnel.health} />
				</div>
				{isRunning && <TunnelMetricsLine metrics={tunnel.metrics} />}
			</div>
			<TunnelActionsMenu tunnel={tunnel} isActiveRow={isActiveRow} />
		</li>
	);
}
