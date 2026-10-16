import type { TunnelStateDto } from '#/daemon/protocol';

const RUNNING_STATE_KINDS: readonly TunnelStateDto['kind'][] = [
	'starting',
	'active',
	'reconnecting',
];

export function isTunnelRunning(state: Readonly<TunnelStateDto>): boolean {
	return RUNNING_STATE_KINDS.includes(state.kind);
}
