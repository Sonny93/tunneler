import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import type { DevslotStaleTunnelDto } from '#/daemon/protocol';
import {
	getTunnelAccessibleEndpointLabel,
	getTunnelEndpointLabel,
} from '#/tunnels/tunnel_endpoint_label';

type DevslotAskItemProps = {
	entry: DevslotStaleTunnelDto;
	isBusy: boolean;
	registerAction: (id: string, element: HTMLButtonElement | null) => void;
	onDelete: (entry: DevslotStaleTunnelDto) => void;
	onKeep: (entry: DevslotStaleTunnelDto) => void;
};

export function DevslotAskItem({
	entry,
	isBusy,
	registerAction,
	onDelete,
	onKeep,
}: Readonly<DevslotAskItemProps>) {
	const { t, i18n } = useLingui();
	const { tunnel, reason } = entry;
	const route = getTunnelAccessibleEndpointLabel(tunnel, i18n);
	const reasonText =
		reason === 'portGone'
			? t`No longer listed by devslot`
			: t`Now belongs to another project`;

	const attachDelete = (element: HTMLButtonElement | null) =>
		registerAction(tunnel.id, element);
	const handleDelete = () => onDelete(entry);
	const handleKeep = () => onKeep(entry);

	return (
		<li className="flex flex-wrap items-center gap-2 rounded-md border border-tn-divider p-2">
			<div className="flex min-w-0 flex-1 flex-col">
				<span className="font-semibold">{getTunnelEndpointLabel(tunnel)}</span>
				<span className="text-sm text-tn-muted">{reasonText}</span>
			</div>
			<Button
				ref={attachDelete}
				size="sm"
				variant="outline"
				color="danger"
				loading={isBusy}
				aria-label={t`Delete ${route}`}
				onClick={handleDelete}
			>
				{t`Delete`}
			</Button>
			<Button
				size="sm"
				variant="outline"
				color="neutral"
				aria-label={t`Keep ${route}`}
				onClick={handleKeep}
			>
				{t`Keep`}
			</Button>
		</li>
	);
}
