import { useLingui } from '@lingui/react/macro';
import { MenuGroup, MenuItem } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';
import { useBulkTunnelActions } from '#/tunnels/selection/use_bulk_tunnel_actions';

type SelectionMenuItemsProps = {
	tunnels: readonly TunnelDto[];
};

/** The bulk actions of the toolbar, for the right-click menu of a selected row. */
export function SelectionMenuItems({
	tunnels,
}: Readonly<SelectionMenuItemsProps>) {
	const { i18n } = useLingui();
	const actions = useBulkTunnelActions(tunnels);

	return (
		<MenuGroup label={describeSelectionCount(tunnels.length, i18n)}>
			{actions.map(({ key, label, icon, isDanger, isDisabled, perform }) => (
				<MenuItem
					key={key}
					icon={icon}
					danger={isDanger}
					disabled={isDisabled}
					onClick={perform}
				>
					{label}
				</MenuItem>
			))}
		</MenuGroup>
	);
}
