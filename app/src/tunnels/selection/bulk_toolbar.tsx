import { Button, IconButton, Toolbar } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { useBulkTunnelActions } from '#/tunnels/selection/use_bulk_tunnel_actions';
import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';

type BulkToolbarProps = {
	selectedTunnels: readonly TunnelDto[];
};

/** The actions on the selected tunnels. A toolbar: one tab stop, arrow keys between buttons. */
export function BulkToolbar({ selectedTunnels }: Readonly<BulkToolbarProps>) {
	const actions = useBulkTunnelActions(selectedTunnels);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);

	return (
		<Toolbar
			aria-label="Actions sur la sélection"
			orientation="horizontal"
			unstyled
			className="flex flex-wrap items-center gap-2"
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
				>
					{label}
				</Button>
			))}
			<IconButton
				icon="i-mdi-close"
				aria-label="Tout désélectionner"
				tooltip
				size="sm"
				variant="ghost"
				color="neutral"
				onClick={clearTunnelSelection}
			/>
		</Toolbar>
	);
}
