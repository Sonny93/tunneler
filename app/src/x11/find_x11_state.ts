import type { X11AnchorDto, X11StateDto } from '#/daemon/protocol';

export const STOPPED_X11_STATE: X11StateDto = { kind: 'stopped' };

/** Only non-stopped anchors are listed, so a host without one is stopped. */
export function findX11State(
	anchors: readonly X11AnchorDto[],
	host: string
): X11StateDto {
	return (
		anchors.find((anchor) => anchor.host === host)?.state ?? STOPPED_X11_STATE
	);
}
