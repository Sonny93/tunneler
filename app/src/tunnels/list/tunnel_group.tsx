import { useId } from 'react';

import { TunnelRow } from '#/tunnels/list/tunnel_row';
import { X11HeaderControls } from '#/x11/x11_header_controls';
import type { TunnelGroup as TunnelGroupModel } from '#/tunnels/group_tunnels_by_host';

type TunnelGroupProps = {
	group: TunnelGroupModel;
	activeRowId: string | null;
	hintId: string;
};

export function TunnelGroup({
	group,
	activeRowId,
	hintId,
}: Readonly<TunnelGroupProps>) {
	const headingId = useId();

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<div className="flex items-center justify-between gap-2">
				<h3 id={headingId} className="text-sm font-semibold text-tn-muted">
					{group.host}
				</h3>
				<X11HeaderControls host={group.host} />
			</div>
			<ul
				role="list"
				aria-describedby={hintId}
				className="m-0 flex list-none flex-col gap-2 p-0"
			>
				{group.tunnels.map((tunnel) => (
					<TunnelRow
						key={tunnel.id}
						tunnel={tunnel}
						isActiveRow={tunnel.id === activeRowId}
					/>
				))}
			</ul>
		</section>
	);
}
