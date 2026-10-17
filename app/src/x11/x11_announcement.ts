import type { X11StateDto } from '#/daemon/protocol';
import { getX11StateDetail, getX11StateLabel } from '#/x11/x11_state_label';

/** "X11 nova : actif (localhost:10.0)". */
export function buildX11Announcement(
	host: string,
	state: Readonly<X11StateDto>
): string {
	const label = getX11StateLabel(state).toLowerCase();
	const detail = getX11StateDetail(state);
	if (detail === null) return `X11 ${host} : ${label}`;
	return `X11 ${host} : ${label} (${detail})`;
}
