import { useLingui } from '@lingui/react/macro';
import { Menu, IconButton } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { TunnelMenuItems } from '#/tunnels/list/tunnel_menu_items';
import { ROVING_CONTROL_ATTRIBUTE } from '#/tunnels/list/use_roving_rows';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type TunnelActionsMenuProps = {
	tunnel: TunnelDto;
	isActiveRow: boolean;
};

export function TunnelActionsMenu({
	tunnel,
	isActiveRow,
}: Readonly<TunnelActionsMenuProps>) {
	const { t, i18n } = useLingui();
	const endpoint = getTunnelAccessibleEndpointLabel(tunnel, i18n);
	const trigger = (
		<IconButton
			icon="i-mdi-dots-vertical"
			aria-label={t`Actions for ${endpoint}`}
			tooltip
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
