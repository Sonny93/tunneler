import { Button } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';
import { useRovingToolbar } from '#/tunnels/selection/use_roving_toolbar';
import { useBulkTunnelActions } from '#/tunnels/selection/use_bulk_tunnel_actions';
import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';

const CLEAR_ITEM_KEY = 'clear';

type BulkToolbarProps = {
	selectedTunnels: readonly TunnelDto[];
};

/** The actions on the selected tunnels. A toolbar: one tab stop, arrow keys between buttons. */
export function BulkToolbar({ selectedTunnels }: Readonly<BulkToolbarProps>) {
	const actions = useBulkTunnelActions(selectedTunnels);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);
	const enabledKeys = [
		...actions.filter(({ isDisabled }) => !isDisabled).map(({ key }) => key),
		CLEAR_ITEM_KEY,
	];
	const { getItemProps, toolbarProps } = useRovingToolbar(enabledKeys);

	return (
		<div
			role="toolbar"
			aria-label="Actions sur la sélection"
			className="flex flex-wrap items-center gap-2"
			{...toolbarProps}
		>
			<span className="text-sm font-semibold">
				{describeSelectionCount(selectedTunnels.length)}
			</span>
			{actions.map(({ key, label, icon, isDanger, isDisabled, perform }) => (
				<Button
					key={key}
					size="sm"
					variant="ghost"
					color={isDanger ? 'danger' : 'neutral'}
					startIcon={icon}
					disabled={isDisabled}
					onClick={perform}
					{...getItemProps(key)}
				>
					{label}
				</Button>
			))}
			<TooltipIconButton
				icon="i-mdi-close"
				aria-label="Tout désélectionner"
				size="sm"
				variant="ghost"
				color="neutral"
				onClick={clearTunnelSelection}
				{...getItemProps(CLEAR_ITEM_KEY)}
			/>
		</div>
	);
}
