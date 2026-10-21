import type { TunnelStateDto } from '#/daemon/protocol';

/** A tunnel in standby is running: its switch is on and it holds its port. */
const RUNNING_STATE_KINDS: readonly TunnelStateDto['kind'][] = [
	'standby',
	'starting',
	'active',
	'reconnecting',
];

export function isTunnelRunning(state: Readonly<TunnelStateDto>): boolean {
	return RUNNING_STATE_KINDS.includes(state.kind);
}
