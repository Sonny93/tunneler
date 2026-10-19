import { IconButton } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { useX11State } from '#/x11/use_x11_state';
import { getX11HeaderStatus } from '#/x11/x11_header_status';

type X11HeaderControlsProps = {
	host: string;
};

/** The X11 button of a host group header, with the anchor status when it is not stopped. */
export function X11HeaderControls({ host }: Readonly<X11HeaderControlsProps>) {
	const openX11 = useUiStore((state) => state.openX11);
	const status = getX11HeaderStatus(useX11State(host));

	const handleOpen = () => openX11(host);

	return (
		<div className="flex items-center gap-2">
			{status !== null && (
				<span className="inline-flex items-center gap-1 text-xs text-tn-muted">
					<span aria-hidden="true" className="i-mdi-monitor size-4 shrink-0" />
					{status}
				</span>
			)}
			<IconButton
				icon="i-mdi-monitor-share"
				aria-label={`X11 pour ${host}`}
				tooltip
				variant="ghost"
				color="neutral"
				size="sm"
				onClick={handleOpen}
			/>
		</div>
	);
}
