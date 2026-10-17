import type { X11StateDto } from '#/daemon/protocol';
import { getX11StateLabel } from '#/x11/x11_state_label';

/** The short status shown in a host header, or `null` while the anchor is stopped. */
export function getX11HeaderStatus(
	state: Readonly<X11StateDto>
): string | null {
	if (state.kind === 'stopped') return null;
	if (state.kind === 'active') return `X11 actif : ${state.display}`;
	return `X11 : ${getX11StateLabel(state).toLowerCase()}`;
}
