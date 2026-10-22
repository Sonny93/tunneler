import { useLingui } from '@lingui/react/macro';
import { useContext, useId, type ChangeEvent } from 'react';
import { Checkbox, ContextMenu, Switch } from '@minimalstuff/ui';

import { cn } from '#/shared/cn';
import type { TunnelDto } from '#/daemon/protocol';
import { TunnelStatusBadge } from '#/shared/status_badge';
import { TemporaryBadge } from '#/shared/temporary_badge';
import { TargetHealthBadge } from '#/shared/health_badge';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { TunnelEndpoint } from '#/tunnels/list/tunnel_endpoint';
import { TunnelTagChips } from '#/tunnels/list/tunnel_tag_chips';
import { TunnelMenuItems } from '#/tunnels/list/tunnel_menu_items';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { TunnelMetricsLine } from '#/tunnels/list/tunnel_metrics_line';
import { TunnelActionsMenu } from '#/tunnels/list/tunnel_actions_menu';
import { SelectionMenuItems } from '#/tunnels/list/selection_menu_items';
import { getTunnelAccessibleName } from '#/tunnels/tunnel_accessible_name';
import { describeTunnelProblems } from '#/tunnels/describe_tunnel_problems';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import { TunnelSelectionContext } from '#/tunnels/selection/tunnel_selection_context';
import {
	ROVING_CONTROL_ATTRIBUTE,
	ROVING_ROW_ATTRIBUTE,
} from '#/tunnels/list/use_roving_rows';

type TunnelRowProps = {
	tunnel: TunnelDto;
	isActiveRow: boolean;
};

const MULTI_SELECTION_MINIMUM = 2;

function isShiftChange(
	event: Readonly<ChangeEvent<HTMLInputElement>>
): boolean {
	return event.nativeEvent instanceof MouseEvent && event.nativeEvent.shiftKey;
}

export function TunnelRow({ tunnel, isActiveRow }: Readonly<TunnelRowProps>) {
	const { t, i18n } = useLingui();
	const { toggleRunning } = useTunnelActions(tunnel);
	const { selectedIds, selectedTunnels, toggle, selectRange } = useContext(
		TunnelSelectionContext
	);
	const isSelected = selectedIds.has(tunnel.id);
	const isInMultiSelection =
		isSelected && selectedTunnels.length >= MULTI_SELECTION_MINIMUM;
	const isRunning = isTunnelRunning(tunnel.state);
	const problemsId = useId();
	const endpoint = getTunnelAccessibleEndpointLabel(tunnel, i18n);
	const problems = describeTunnelProblems(tunnel, i18n);

	const handleSelectionChange = (event: ChangeEvent<HTMLInputElement>) => {
		if (isShiftChange(event)) selectRange(tunnel.id);
		else toggle(tunnel.id);
	};

	const menuItems = isInMultiSelection ? (
		<SelectionMenuItems tunnels={selectedTunnels} />
	) : (
		<TunnelMenuItems tunnel={tunnel} />
	);

	return (
		<li {...{ [ROVING_ROW_ATTRIBUTE]: tunnel.id }}>
			<ContextMenu
				items={menuItems}
				aria-label={t`Actions for ${endpoint}`}
				className={cn(
					'flex flex-wrap items-center gap-x-3 gap-y-2 px-3 py-2 hover:bg-tn-surface-raised',
					isSelected && 'bg-tn-selected'
				)}
			>
				<Checkbox
					aria-label={t`Select ${endpoint}`}
					checked={isSelected}
					onChange={handleSelectionChange}
					tabIndex={isActiveRow ? 0 : -1}
					wrapperClassName="shrink-0"
					{...{ [ROVING_CONTROL_ATTRIBUTE]: 'select' }}
				/>
				<Switch
					aria-label={getTunnelAccessibleName(tunnel, i18n)}
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
				<div className="flex min-w-min flex-1 flex-col gap-1">
					<TunnelEndpoint
						tunnel={tunnel}
						className="font-semibold whitespace-nowrap text-tn-text"
					/>
					<TunnelTagChips tags={tunnel.tags} />
				</div>
				<div className="flex min-w-0 flex-1 basis-56 flex-col items-start gap-1">
					<div className="flex w-full min-w-0 flex-nowrap gap-1">
						{tunnel.isTemporary && <TemporaryBadge />}
						<TunnelStatusBadge state={tunnel.state} isDetailTruncated />
						<TargetHealthBadge health={tunnel.health} isDetailTruncated />
					</div>
					{isRunning && <TunnelMetricsLine metrics={tunnel.metrics} />}
				</div>
				<TunnelActionsMenu tunnel={tunnel} isActiveRow={isActiveRow} />
			</ContextMenu>
		</li>
	);
}
