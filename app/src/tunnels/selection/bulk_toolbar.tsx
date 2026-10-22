import { useLingui } from '@lingui/react/macro';
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
	const { t, i18n } = useLingui();
	const actions = useBulkTunnelActions(selectedTunnels);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);

	return (
		<Toolbar
			aria-label={t`Actions on the selection`}
			orientation="horizontal"
			unstyled
			className="flex flex-wrap items-center gap-2"
		>
			<span className="text-sm font-semibold">
				{describeSelectionCount(selectedTunnels.length, i18n)}
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
				aria-label={t`Deselect all`}
				tooltip
				size="sm"
				variant="ghost"
				color="neutral"
				onClick={clearTunnelSelection}
			/>
		</Toolbar>
	);
}
