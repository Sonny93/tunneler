import { Menu } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';
import { TunnelMenuItems } from '#/tunnels/list/tunnel_menu_items';
import { ROVING_CONTROL_ATTRIBUTE } from '#/tunnels/list/use_roving_rows';

type TunnelActionsMenuProps = {
	tunnel: TunnelDto;
	isActiveRow: boolean;
};

export function TunnelActionsMenu({
	tunnel,
	isActiveRow,
}: Readonly<TunnelActionsMenuProps>) {
	const trigger = (
		<TooltipIconButton
			icon="i-mdi-dots-vertical"
			aria-label={`Actions pour ${tunnel.name}`}
			variant="ghost"
			color="neutral"
			size="sm"
			className="size-8"
			tabIndex={isActiveRow ? 0 : -1}
			{...{ [ROVING_CONTROL_ATTRIBUTE]: 'actions' }}
		/>
	);

	return (
		<Menu trigger={trigger} align="end">
			<TunnelMenuItems tunnel={tunnel} />
		</Menu>
	);
}
