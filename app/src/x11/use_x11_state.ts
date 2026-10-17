import { findX11State } from '#/x11/find_x11_state';
import type { X11StateDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';

export const useX11State = (host: string): X11StateDto =>
	useDaemonStore((state) => findX11State(state.x11Anchors, host));
